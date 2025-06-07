use serde::{Deserialize, Serialize};
use std::{
    io::{BufRead, BufReader, Cursor, Write},
    sync::{Arc, Mutex},
};
use tokio::time::{sleep, timeout, Duration};

pub struct BufferWriter {
    pub buffer: Arc<Mutex<Vec<u8>>>,
}

impl Write for BufferWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut buffer = self.buffer.lock().unwrap();
        buffer.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[allow(async_fn_in_trait)]
pub trait Runner {
    fn log_buffer(&self) -> Arc<Mutex<Vec<u8>>>;

    fn log_filter(&self, log: &Log) -> bool;

    fn name(&self) -> &str;

    fn get_logs(&self) -> impl Iterator<Item = Log> {
        let log_buffer = self.log_buffer();
        let buffer = log_buffer.lock().unwrap();
        let log_output = String::from_utf8(buffer.clone()).unwrap();

        let cursor = Cursor::new(log_output);
        let reader = BufReader::new(cursor);

        reader.lines().filter_map(|l| {
            if let Ok(l) = l {
                match serde_json::from_str::<Log>(&l) {
                    Ok(log) => Some(log),
                    Err(_) => None,
                }
            } else {
                None
            }
        })
    }

    fn log_output(&self) -> String {
        let logs = self.get_logs();
        serde_json::to_string_pretty(&logs.collect::<Vec<Log>>()).unwrap()
    }

    async fn assert_info_log_entry(&self, entry: &str) {
        self.assert_log(entry, tracing::Level::INFO, Eval::Equals)
            .await;
    }

    async fn assert_info_log_contains(&self, entry: &str) {
        self.assert_log(entry, tracing::Level::INFO, Eval::Contains)
            .await;
    }

    async fn assert_error_log_entry(&self, entry: &str) {
        self.assert_log(entry, tracing::Level::ERROR, Eval::Equals)
            .await;
    }

    async fn assert_log(&self, entry: &str, level: tracing::Level, eval: Eval) {
        let predicate: Box<dyn Fn(&String) -> bool> = match eval {
            Eval::Contains => {
                let entry_clone = entry.to_string();
                Box::new(move |msg: &String| msg.contains(&entry_clone))
            }
            Eval::Equals => {
                let entry_clone = entry.to_string();
                Box::new(move |msg: &String| msg == &entry_clone)
            }
        };

        let duration = Duration::from_secs(2);
        if timeout(duration, self.parse_logs(predicate, level))
            .await
            .is_err()
        {
            let log_buffer = self.log_buffer();
            let buffer = log_buffer.lock().unwrap();
            let log_output = String::from_utf8(buffer.clone()).unwrap();
            panic!(
                "Logs:\n{}\nFailed to find log entry for {}: {}",
                log_output,
                self.name(),
                entry
            );
        }
    }

    async fn parse_logs(&self, predicate: Box<dyn Fn(&String) -> bool>, level: tracing::Level) {
        let mut logs: Vec<Log> = vec![];

        while logs.is_empty() {
            logs = self
                .get_logs()
                .filter(|log| log.level() == level.as_str())
                .filter(|log| self.log_filter(log))
                .filter(|log| match &log.fields {
                    Fields::Message { message } => predicate(message),
                    _ => false,
                })
                .collect::<Vec<Log>>();

            sleep(Duration::from_millis(100)).await;
        }
    }
}

pub enum Eval {
    Contains,
    Equals,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Log {
    level: String,
    fields: Fields,
    target: String,
    span: Span,
}

#[derive(Deserialize, Serialize, Debug)]
struct Span {
    label: String,
    name: String,
}

impl Log {
    pub fn target(&self) -> &str {
        &self.target
    }

    pub fn label(&self) -> &str {
        &self.span.label
    }

    pub fn level(&self) -> &str {
        &self.level
    }
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(untagged)]
enum Fields {
    #[allow(unused)]
    LocalPeerId {
        local_peer_id: String,
    },
    Message {
        message: String,
    },
}
