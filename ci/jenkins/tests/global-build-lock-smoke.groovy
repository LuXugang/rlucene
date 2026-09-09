// Isolated Jenkins init script, NEVER run against an operational Jenkins home.
// See README.md in this directory. Uses real Pipelines, sh processes and two
// nodes, but replaces repository checkout/Cargo/mail with lightweight probes.
import hudson.model.Cause
import hudson.model.CauseAction
import hudson.model.Node
import hudson.model.Result
import hudson.model.TaskListener
import hudson.model.queue.QueueTaskDispatcher
import hudson.slaves.DumbSlave
import hudson.slaves.JNLPLauncher
import jenkins.model.Jenkins
import org.jenkinsci.plugins.pipeline.modeldefinition.parser.Converter
import org.jenkinsci.plugins.workflow.cps.CpsFlowDefinition
import org.jenkinsci.plugins.workflow.job.WorkflowJob
import org.jenkins.plugins.lockableresources.LockableResourcesManager
import java.util.concurrent.TimeUnit

assert System.getenv('RLUCENE_GLOBAL_LOCK_SMOKE') == '1'
def j = Jenkins.get()
assert j.rootDir.name == 'home' && j.rootDir.parentFile.name.startsWith('rlucene-global-lock-test.')
assert j.getAllItems(WorkflowJob.class).isEmpty()
def repo = new File(System.getenv('RLUCENE_LOCK_TEST_REPO'))
def output = new File(j.rootDir.parentFile, 'result.log')
def shared = new File(j.rootDir.parentFile, 'probe')
shared.mkdirs()
def record = { message -> output.append(message.toString() + '\n') }
def await = { String description, Closure condition ->
  long end = System.currentTimeMillis() + 60000
  while (!condition()) {
    assert System.currentTimeMillis() < end : 'Timed out: ' + description
    Thread.sleep(100)
  }
}
def trigger = { job ->
  def future = job.scheduleBuild2(0, new CauseAction(new Cause.UserIdCause()))
  assert future != null
  future
}
def create = { String name, String script ->
  def job = j.createProject(WorkflowJob.class, name)
  job.definition = new CpsFlowDefinition(script, true)
  job.save()
  job
}
def activeExecutors = {
  j.computers.collectMany { it.executors }.count { it.currentExecutable != null }
}
def trace = new File(shared, 'trace')
def probeSteps = { String action -> """
          sh '''
            set -eu
            mkdir "${shared}/busy"
            printf '%s' "\$JOB_NAME" > "${shared}/owner"
            printf 'ENTER %s\\n' "\$JOB_NAME" >> "${shared}/trace"
          '''
          ${action}
""" }
def probePost = """
        always {
          sh '''
            set -eu
            if test -f "${shared}/owner" && test "\$(cat '${shared}/owner')" = "\$JOB_NAME"; then
              printf 'POST %s\\n' "\$JOB_NAME" >> "${shared}/trace"
              sleep 1
              rm "${shared}/owner"
              rmdir "${shared}/busy"
              printf 'LEAVE %s\\n' "\$JOB_NAME" >> "${shared}/trace"
            fi
          '''
        }
"""
def templates = [:]
['ci': 'Jenkinsfile', 'commit': 'commit/Jenkinsfile', 'pr': 'pr/Jenkinsfile',
 'nightly': 'manual/Jenkinsfile', 'monster': 'manual/Jenkinsfile'].each { name, path ->
  String source = new File(repo, 'ci/jenkins/' + path).text
  // Validate the entire unchanged production body, not only the reduced probe.
  assert Converter.scriptToPipelineDef(source) != null
  assert source.count("lock(resource: 'rlucene-vm-build')") == 1
  assert source.contains('agent none') && source.contains('disableConcurrentBuilds()')
  int stages = source.indexOf('      stages {')
  int post = source.indexOf('      post {')
  assert stages > 0 && post > stages
  String prefix = source.substring(0, stages)
    .replaceAll(/(?s)  triggers \{.*?\n  \}\n/, '')
  templates[name] = { action -> prefix + """
      stages {
        stage('Probe') {
          steps { ${probeSteps(action)} }
        }
      }
      post { ${probePost} }
    }
  }
}
""" }
  record('VALIDATED ' + path)
}

j.setNumExecutors(2)
def node = new DumbSlave('smoke-pr-agent', new File(j.rootDir.parentFile, 'agent').path, new JNLPLauncher())
node.setNumExecutors(1)
node.setMode(Node.Mode.EXCLUSIVE)
node.setLabelString('rlucene-pr')
j.addNode(node)
def remoting = new File(System.getenv('RLUCENE_LOCK_TEST_REMOTING'))
assert remoting.isFile()
def process = new ProcessBuilder(new File(System.getProperty('java.home'), 'bin/java').path,
  '-Xmx256m', '-jar', remoting.path).redirectError(new File(j.rootDir.parentFile, 'agent.log')).start()
node.toComputer().setChannel(process.inputStream, process.outputStream, TaskListener.NULL, null)

Thread.start('global-lock-smoke') {
  try {
    await('Jenkins ready') { j.initLevel == hudson.init.InitMilestone.COMPLETED && !node.toComputer().offline }
    def manager = LockableResourcesManager.get()
    def initHook = new File(repo, 'ci/jenkins/deployment/init.groovy.d/rlucene-global-build-lock.groovy.override')
    def retireHook = new File(repo, 'ci/jenkins/deployment/init.groovy.d/rlucene-ci-idle-gate.groovy.override')
    def baselineHook = new File(System.getenv('RLUCENE_LOCK_TEST_BASELINE_HOOK'))
    evaluate(baselineHook)
    assert QueueTaskDispatcher.all().count { it.class.name == 'RluceneCiIdleQueueDispatcher' } == 1

    // Deterministic old behavior: CI starts first; a different job can still
    // enter on the PR node and collide with the already-running CI workload.
    def baseline = { label, action -> """
pipeline {
  agent { label '${label}' }
  stages { stage('Probe') { steps { ${probeSteps(action)} } } }
  post { ${probePost} }
}
""" }
    def oldCi = create('rlucene-ci', baseline('built-in', "sleep time: 8, unit: 'SECONDS'"))
    def oldOther = create('baseline-commit', baseline('rlucene-pr', "echo 'unexpected overlap'"))
    def oldCiFuture = trigger(oldCi)
    await('old CI enters') { trace.exists() && trace.text.contains('ENTER rlucene-ci') }
    assert trigger(oldOther).get(60, TimeUnit.SECONDS).result == Result.FAILURE
    assert oldCiFuture.get(60, TimeUnit.SECONDS).result == Result.SUCCESS
    record('BASELINE_REPRODUCED other job attempted work during CI; atomic mkdir rejected overlap')
    evaluate(retireHook)
    assert !QueueTaskDispatcher.all().any { it.class.name == 'RluceneCiIdleQueueDispatcher' }
    evaluate(initHook)
    trace.text = ''

    // All five production wrappers contend at once across both nodes. The
    // holder also exercises init-hook idempotence without resetting ownership.
    def jobs = templates.collectEntries { name, template ->
      [(name): create('locked-' + name, template("sleep time: 4, unit: 'SECONDS'"))]
    }
    def futures = jobs.collectEntries { name, job -> [(name): trigger(job)] }
    await('one holder and four waiters') {
      jobs.values().every { it.isBuilding() } && manager.queuedContexts.size() == 4 &&
        trace.text.contains('ENTER locked-')
    }
    assert activeExecutors() == 1
    def resource = manager.fromName('rlucene-vm-build')
    assert resource.locked && !resource.ephemeral
    String holder = resource.getBuild().externalizableId
    assert holder != null
    evaluate(initHook)
    assert resource.locked && resource.getBuild().externalizableId == holder
    assert manager.queuedContexts.size() == 4
    record('FIVE_JOB_CONTENTION one executor, four lock waiters, initializer preserves holder/queue')
    futures.each { name, future -> assert future.get(90, TimeUnit.SECONDS).result == Result.SUCCESS }
    List<String> lines = trace.readLines()
    assert lines.size() == 15
    lines.collate(3).each { chunk ->
      String owner = chunk[0].substring('ENTER '.length())
      assert chunk == ['ENTER ' + owner, 'POST ' + owner, 'LEAVE ' + owner]
    }
    assert !resource.locked && manager.queuedContexts.isEmpty()
    assert manager.fromName('rlucene-vm-build') != null
    record('SERIAL_AND_POST_PASSED ' + lines.findAll { it.startsWith('ENTER ') })

    // Reproduce the original start order with the fixed production wrappers.
    oldCi.definition = new CpsFlowDefinition(templates.ci("sleep time: 4, unit: 'SECONDS'"), true)
    oldCi.save()
    def first = trigger(oldCi)
    await('fixed CI owns lock') {
      resource.locked && resource.getBuild()?.externalizableId?.startsWith('rlucene-ci#') &&
        trace.text.contains('ENTER rlucene-ci')
    }
    def second = trigger(jobs.commit)
    await('fixed commit waits') { manager.queuedContexts.size() == 1 }
    assert activeExecutors() == 1
    assert first.get(60, TimeUnit.SECONDS).result == Result.SUCCESS
    assert second.get(60, TimeUnit.SECONDS).result == Result.SUCCESS
    record('SAME_SCENARIO_FIXED CI then commit execute without overlap')

    // Failure/timeout/abort must clean up under the lock and unblock the next
    // node; cancelling a waiter must not steal or release the holder's lock.
    [failure: "error('expected probe failure')", timeout: "sleep time: 30, unit: 'SECONDS'",
     abort: "sleep time: 30, unit: 'SECONDS'", cancelWaiter: "sleep time: 6, unit: 'SECONDS'"].each { mode, action ->
      String source = templates.ci(action)
      if (mode == 'timeout') source = source.replace("timeout(time: 30, unit: 'MINUTES')", "timeout(time: 3, unit: 'SECONDS')")
      def owner = create('owner-' + mode, source)
      def ownerFuture = trigger(owner)
      await('owner ' + mode) { trace.text.contains('ENTER owner-' + mode) }
      def follower = create('follower-' + mode, templates.pr("echo 'follower'"))
      def followerFuture = trigger(follower)
      if (mode in ['abort', 'cancelWaiter']) {
        await('waiter ' + mode) { manager.queuedContexts.size() == 1 }
        if (mode == 'abort') owner.lastBuild.doStop()
        else {
          follower.lastBuild.doStop()
          assert followerFuture.get(60, TimeUnit.SECONDS).result == Result.ABORTED
          assert resource.locked && resource.getBuild().externalizableId.startsWith('owner-cancelWaiter#')
        }
      }
      Result expected = mode == 'failure' ? Result.FAILURE :
        (mode in ['timeout', 'abort'] ? Result.ABORTED : Result.SUCCESS)
      assert ownerFuture.get(60, TimeUnit.SECONDS).result == expected
      if (mode != 'cancelWaiter') assert followerFuture.get(60, TimeUnit.SECONDS).result == Result.SUCCESS
      assert !resource.locked && manager.queuedContexts.isEmpty()
      assert !new File(shared, 'busy').exists()
      record('RECOVERY_PASSED ' + mode)
    }
    def longOwner = create('wait-budget-owner', templates.ci("sleep time: 9, unit: 'SECONDS'"))
    def longFuture = trigger(longOwner)
    await('wait-budget owner enters') { trace.text.contains('ENTER wait-budget-owner') }
    def shortWaiter = create('wait-budget-follower', templates.pr("echo 'short execution'")
      .replace("timeout(time: 35, unit: 'MINUTES')", "timeout(time: 5, unit: 'SECONDS')"))
    def shortFuture = trigger(shortWaiter)
    await('wait-budget follower waits') { manager.queuedContexts.size() == 1 }
    Thread.sleep(6000)
    assert shortWaiter.isBuilding() && manager.queuedContexts.size() == 1
    assert longFuture.get(60, TimeUnit.SECONDS).result == Result.SUCCESS
    assert shortFuture.get(60, TimeUnit.SECONDS).result == Result.SUCCESS
    record('WAIT_EXCLUDED_FROM_TIMEOUT_PASSED waited over 5s; 5s execution budget still succeeded')
    record('SUCCESS')
  } catch (Throwable failure) {
    def writer = new StringWriter()
    failure.printStackTrace(new PrintWriter(writer))
    record('FAILED ' + writer)
  } finally {
    j.getAllItems(WorkflowJob.class).each { job ->
      job.setDisabled(true)
      job.save()
      job.builds.findAll { it.isBuilding() }.each { it.doStop() }
    }
    await('test builds stopped') { !j.getAllItems(WorkflowJob.class).any { it.isBuilding() } }
    process.destroy()
  }
}
