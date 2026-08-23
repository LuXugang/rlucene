/*
 * Licensed to the Apache Software Foundation (ASF) under one or more
 * contributor license agreements.  See the NOTICE file distributed with
 * this work for additional information regarding copyright ownership.
 * The ASF licenses this file to You under the Apache License, Version 2.0
 * (the "License"); you may not use this file except in compliance with
 * the License.  You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */
use crate::core::document::document::Document;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_reader_context::{IRCLeafReader, IndexReaderContext};
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::index::standard_directory_reader::StandardDirectoryReader;
use crate::core::search::collector::Collector;
use crate::core::search::collector_manager::CollectorManager;
use crate::core::search::index_searcher::{IndexSearcher, IndexSearcherHook};
use crate::core::search::leaf_collector::LeafCollector;
use crate::core::search::match_all_docs_query::MatchAllDocsQuery;
use crate::core::search::scorable::Scorable;
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::task_executor::TaskExecutor;
use crate::core::search::weight::Weight;
use crate::core::store::directory::DirEnum;
use crate::core::util::close::CloseableRef;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::io_utils::IOUtils;
use crate::test_framework::core::index::random_index_writer::RandomIndexWriter;
use crate::test_framework::core::search::test_index_searcher::GetSlicesIndexSearcher;
use crate::test_framework::core::util::lucene_test_case::{
  new_directory_shared, new_search_executor, random,
};
use rand::RngExt;
use rayon::ThreadPool;
use std::fmt::{Display, Formatter};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Barrier, LazyLock};
use std::thread;
use std::time::Duration;

#[allow(dead_code)] // for quick search
struct TestTaskExecutor;

static EXECUTOR: LazyLock<Arc<ThreadPool>> =
  LazyLock::new(|| new_search_executor(1).expect("failed to create the TestTaskExecutor executor"));

#[test]
fn test_unwrap_io_exception_from_execution_exception() -> Result<()> {
  let task_executor = TaskExecutor::new(EXECUTOR.clone());
  let error = task_executor
    .invoke_all(vec![|| -> Result<()> {
      Err(LuceneError::io(std::io::Error::other("io exception")))
    }])
    .expect_err("the callable must fail");
  match error {
    LuceneError::Io { source, .. } | LuceneError::IoWithPath { source, .. } => {
      assert_eq!("io exception", source.to_string());
    },
    error => panic!("expected an I/O error, got {error}"),
  }
  Ok(())
}

#[test]
fn test_unwrap_runtime_exception_from_execution_exception() -> Result<()> {
  let task_executor = TaskExecutor::new(EXECUTOR.clone());
  let error = task_executor
    .invoke_all(vec![|| -> Result<()> {
      Err(LuceneError::illegal_state("runtime"))
    }])
    .expect_err("the callable must fail");
  assert!(matches!(
    error,
    LuceneError::IllegalState(ref error) if error.message == "runtime"
  ));
  Ok(())
}

#[test]
fn test_unwrap_error_from_execution_exception() -> Result<()> {
  let task_executor = TaskExecutor::new(EXECUTOR.clone());
  let error = catch_unwind(AssertUnwindSafe(|| {
    let _ = task_executor.invoke_all(vec![|| -> Result<()> { panic!("oom") }]);
  }))
  .expect_err("the callable must panic");
  assert_eq!("oom", LuceneError::panic_payload_message(error.as_ref()));
  Ok(())
}

#[test]
#[ignore = "Java-only: Rust has no checked Exception category that IOUtils.rethrowAlways would wrap in RuntimeException"]
fn test_unwrapped_exceptions() {}

#[test]
fn test_invoke_all_from_task_does_not_deadlock_same_searcher() -> Result<()> {
  let mut random = random();
  do_test_invoke_all_from_task_does_not_deadlock_same_searcher(
    &mut random,
    Some(EXECUTOR.clone()),
  )?;
  do_test_invoke_all_from_task_does_not_deadlock_same_searcher(&mut random, None)?;
  EXECUTOR.install(|| {
    do_test_invoke_all_from_task_does_not_deadlock_same_searcher(
      &mut random,
      Some(EXECUTOR.clone()),
    )
  })
}

fn do_test_invoke_all_from_task_does_not_deadlock_same_searcher<R>(
  random: &mut R,
  executor: Option<Arc<ThreadPool>>,
) -> Result<()>
where
  R: rand::Rng + ?Sized,
{
  let dir = new_directory_shared(random)?;
  let writer = RandomIndexWriter::new(random, dir.clone())?;
  for _ in 0..500 {
    writer.add_document(random, Document::new())?;
  }
  let reader = Arc::new(writer.get_reader(random)?);

  let body_result = catch_unwind(AssertUnwindSafe(|| -> Result<()> {
    let context = reader.clone().get_context()?;
    let searcher = match executor {
      Some(executor) => IndexSearcher::with_executor(context, executor)?,
      None => IndexSearcher::new(context)?,
    }
    .with_hook(IndexSearcherHook::GetSlices(GetSlicesIndexSearcher));
    let collector_manager = NestedInvocationCollectorManager {
      invocation: NestedInvocation::SameSearcher(searcher.get_task_executor()),
    };
    searcher.search_with_collector_manager(MatchAllDocsQuery::new(), &collector_manager)
  }));
  let close_result = catch_unwind(AssertUnwindSafe(|| -> Result<()> {
    let close_result = IOUtils::use_or_suppress_result(reader.close(), writer.close(random));
    IOUtils::use_or_suppress_result(close_result, dir.close())
  }));
  IOUtils::use_or_suppress_caught_result(body_result, close_result)
}

#[test]
fn test_invoke_all_from_task_does_not_deadlock_multiple_searchers() -> Result<()> {
  let mut random = random();
  do_test_invoke_all_from_task_does_not_deadlock_multiple_searchers(
    &mut random,
    Some(EXECUTOR.clone()),
  )?;
  do_test_invoke_all_from_task_does_not_deadlock_multiple_searchers(&mut random, None)?;
  EXECUTOR.install(|| {
    do_test_invoke_all_from_task_does_not_deadlock_multiple_searchers(
      &mut random,
      Some(EXECUTOR.clone()),
    )
  })
}

fn do_test_invoke_all_from_task_does_not_deadlock_multiple_searchers<R>(
  random: &mut R,
  executor: Option<Arc<ThreadPool>>,
) -> Result<()>
where
  R: rand::Rng + ?Sized,
{
  let dir = new_directory_shared(random)?;
  let writer = RandomIndexWriter::new(random, dir.clone())?;
  for _ in 0..500 {
    writer.add_document(random, Document::new())?;
  }
  let reader = Arc::new(writer.get_reader(random)?);

  let body_result = catch_unwind(AssertUnwindSafe(|| -> Result<()> {
    let context = reader.clone().get_context()?;
    let searcher = match executor.clone() {
      Some(executor) => IndexSearcher::with_executor(context, executor)?,
      None => IndexSearcher::new(context)?,
    }
    .with_hook(IndexSearcherHook::GetSlices(GetSlicesIndexSearcher));
    let collector_manager = NestedInvocationCollectorManager {
      invocation: NestedInvocation::MultipleSearchers {
        reader: reader.clone(),
        executor,
      },
    };
    searcher.search_with_collector_manager(MatchAllDocsQuery::new(), &collector_manager)
  }));
  let close_result = catch_unwind(AssertUnwindSafe(|| -> Result<()> {
    let close_result = IOUtils::use_or_suppress_result(reader.close(), writer.close(random));
    IOUtils::use_or_suppress_result(close_result, dir.close())
  }));
  IOUtils::use_or_suppress_caught_result(body_result, close_result)
}

#[test]
fn test_invoke_all_does_not_leave_tasks_behind() -> Result<()> {
  let tasks_executed = AtomicUsize::new(0);
  let task_executor = TaskExecutor::direct();
  let tasks = (0..100)
    .map(|index| {
      let tasks_executed = &tasks_executed;
      move || -> Result<()> {
        tasks_executed.fetch_add(1, Ordering::SeqCst);
        if index == 0 {
          Err(LuceneError::illegal_state("exception"))
        } else {
          panic!("must not be called since the first task failing cancels all subsequent tasks")
        }
      }
    })
    .collect::<Vec<_>>();
  let error = task_executor
    .invoke_all(tasks)
    .expect_err("the first callable must fail");
  assert!(matches!(error, LuceneError::IllegalState(_)));
  assert_eq!(1, tasks_executed.load(Ordering::SeqCst));
  Ok(())
}

/// Ensures that invoke_all catches all exceptions thrown by callables and adds subsequent ones as
/// suppressed exceptions to the first one caught.
#[test]
fn test_invoke_all_catches_multiple_exceptions() -> Result<()> {
  let task_executor = TaskExecutor::new(new_search_executor(2)?);
  let barrier = Arc::new(Barrier::new(2));
  let tasks = ["exception A", "exception B"]
    .into_iter()
    .map(|message| {
      let barrier = barrier.clone();
      move || -> Result<()> {
        barrier.wait();
        Err(LuceneError::illegal_state(message))
      }
    })
    .collect::<Vec<_>>();

  let error = task_executor
    .invoke_all(tasks)
    .expect_err("both callables must fail");
  let suppressed = error
    .get_suppressed()?
    .expect("the second exception must be suppressed");
  if error.to_string().contains("exception A") {
    assert!(suppressed.to_string().contains("exception B"));
  } else {
    assert!(error.to_string().contains("exception B"));
    assert!(suppressed.to_string().contains("exception A"));
  }
  Ok(())
}

#[test]
fn test_cancel_tasks_on_exception() -> Result<()> {
  let mut random = random();
  let task_executor = TaskExecutor::direct();
  let num_tasks = random.random_range(10..50);
  let throwing_task = random.random_range(0..num_tasks);
  let error = random.random_bool(0.5);
  let executed_tasks = AtomicUsize::new(0);
  let tasks = (0..num_tasks)
    .map(|index| {
      let executed_tasks = &executed_tasks;
      move || {
        if index == throwing_task {
          if error {
            panic!("error");
          }
          return Err(LuceneError::illegal_state("exception"));
        }
        assert!(index < throwing_task, "task should not have started");
        executed_tasks.fetch_add(1, Ordering::SeqCst);
        Ok(())
      }
    })
    .collect::<Vec<_>>();

  if error {
    let panic = catch_unwind(AssertUnwindSafe(|| task_executor.invoke_all(tasks)))
      .expect_err("the callable must panic");
    assert_eq!("error", LuceneError::panic_payload_message(panic.as_ref()));
  } else {
    let error = task_executor
      .invoke_all(tasks)
      .expect_err("the callable must fail");
    assert!(error.get_suppressed()?.is_none());
  }
  assert_eq!(throwing_task, executed_tasks.load(Ordering::SeqCst));
  Ok(())
}

#[test]
#[ignore = "Java-only: Rayon scoped tasks are accepted by a live ThreadPool and do not expose RejectedExecutionException"]
fn test_task_rejection_does_not_fail_execution() {}

#[test]
fn test_results_keep_callable_order() -> Result<()> {
  let task_executor = TaskExecutor::new(new_search_executor(2)?);
  let tasks = (0..10)
    .map(|index| {
      move || {
        thread::sleep(Duration::from_millis((10 - index) as u64));
        Ok(index)
      }
    })
    .collect::<Vec<_>>();
  assert_eq!(
    (0..10).collect::<Vec<_>>(),
    task_executor.invoke_all(tasks)?
  );
  Ok(())
}

#[derive(Clone)]
enum NestedInvocation<'a> {
  SameSearcher(&'a TaskExecutor),
  MultipleSearchers {
    reader: Arc<StandardDirectoryReader<DirEnum>>,
    executor: Option<Arc<ThreadPool>>,
  },
}

struct NestedInvocationCollectorManager<'a> {
  invocation: NestedInvocation<'a>,
}

impl<'a> CollectorManager for NestedInvocationCollectorManager<'a> {
  type C = NestedInvocationCollector<'a>;
  type T = ();

  fn new_collector(&self) -> Result<Self::C> {
    Ok(NestedInvocationCollector {
      invocation: self.invocation.clone(),
    })
  }

  fn reduce(&self, _collectors: Vec<Self::C>) -> Result<Self::T> {
    Ok(())
  }
}

struct NestedInvocationCollector<'a> {
  invocation: NestedInvocation<'a>,
}

impl Collector for NestedInvocationCollector<'_> {
  type LeafCollector<'a, IRC>
    = &'a mut Self
  where
    Self: 'a,
    IRC: IndexReaderContext + 'a;

  fn get_leaf_collector<'a, W, IRC>(
    &'a mut self,
    _context: &LeafReaderContext<IRCLeafReader<IRC>>,
    _weight: Option<&W>,
    _searcher: &IndexSearcher<IRC>,
  ) -> Result<Self::LeafCollector<'a, IRC>>
  where
    IRC: IndexReaderContext,
    W: Weight<IRC> + ?Sized,
  {
    Ok(self)
  }

  fn score_mode(&self) -> ScoreMode {
    ScoreMode::Complete
  }
}

impl LeafCollector for NestedInvocationCollector<'_> {
  fn set_scorer(&mut self, _scorer: &mut dyn Scorable) -> Result<()> {
    match &self.invocation {
      NestedInvocation::SameSearcher(task_executor) => {
        task_executor.invoke_all(vec![|| -> Result<()> {
          task_executor.invoke_all(vec![|| -> Result<()> { Ok(()) }])?;
          Ok(())
        }])?;
      },
      NestedInvocation::MultipleSearchers { reader, executor } => {
        let context = reader.clone().get_context()?;
        let searcher = match executor {
          Some(executor) => IndexSearcher::with_executor(context, executor.clone())?,
          None => IndexSearcher::new(context)?,
        };
        searcher
          .get_task_executor()
          .invoke_all(vec![|| -> Result<()> { Ok(()) }])?;
      },
    }
    Ok(())
  }

  fn collect(&mut self, _doc: i32, _scorer: &mut dyn Scorable) -> Result<()> {
    Ok(())
  }
}

impl Display for NestedInvocationCollector<'_> {
  fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
    formatter.write_str("NestedInvocationCollector")
  }
}
