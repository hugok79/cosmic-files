use cosmic::iced::futures::{channel::mpsc, SinkExt};
use std::{fs, path::PathBuf};

use crate::app::Message;

fn err_str<T: ToString>(err: T) -> String {
    err.to_string()
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum Operation {
    /// Copy items
    Copy {
        paths: Vec<PathBuf>,
        to: PathBuf,
    },
    /// Move items to the trash
    Delete {
        paths: Vec<PathBuf>,
    },
    /// Move items
    Move {
        paths: Vec<PathBuf>,
        to: PathBuf,
    },
    NewFile {
        path: PathBuf,
    },
    NewFolder {
        path: PathBuf,
    },
    Rename {
        from: PathBuf,
        to: PathBuf,
    },
    /// Restore a path from the trash
    Restore {
        paths: Vec<trash::TrashItem>,
    },
}

impl Operation {
    /// Perform the operation
    pub async fn perform(self, id: u64, msg_tx: &mut mpsc::Sender<Message>) -> Result<(), String> {
        let _ = msg_tx.send(Message::PendingProgress(id, 0.0)).await;

        //TODO: IF ERROR, RETURN AN Operation THAT CAN UNDO THE CURRENT STATE
        //TODO: SAFELY HANDLE CANCEL
        match self {
            Self::Delete { paths } => {
                let total = paths.len();
                let mut count = 0;
                for path in paths {
                    tokio::task::spawn_blocking(|| trash::delete(path))
                        .await
                        .map_err(err_str)?
                        .map_err(err_str)?;
                    count += 1;
                    let _ = msg_tx
                        .send(Message::PendingProgress(
                            id,
                            100.0 * (count as f32) / (total as f32),
                        ))
                        .await;
                }
            }
            Self::NewFolder { path } => {
                tokio::task::spawn_blocking(|| fs::create_dir(path))
                    .await
                    .map_err(err_str)?
                    .map_err(err_str)?;
                let _ = msg_tx.send(Message::PendingProgress(id, 100.0)).await;
            }
            Self::NewFile { path } => {
                tokio::task::spawn_blocking(|| fs::File::create(path))
                    .await
                    .map_err(err_str)?
                    .map_err(err_str)?;
                let _ = msg_tx.send(Message::PendingProgress(id, 100.0)).await;
            }
            Self::Rename { from, to } => {
                tokio::task::spawn_blocking(|| fs::rename(from, to))
                    .await
                    .map_err(err_str)?
                    .map_err(err_str)?;
                let _ = msg_tx.send(Message::PendingProgress(id, 100.0)).await;
            }
            Self::Restore { paths } => {
                let total = paths.len();
                let mut count = 0;
                for path in paths {
                    tokio::task::spawn_blocking(|| trash::os_limited::restore_all([path]))
                        .await
                        .map_err(err_str)?
                        .map_err(err_str)?;
                    count += 1;
                    let _ = msg_tx
                        .send(Message::PendingProgress(
                            id,
                            100.0 * (count as f32) / (total as f32),
                        ))
                        .await;
                }
            }
            _ => {
                return Err("not implemented".to_string());
            }
        }

        let _ = msg_tx.send(Message::PendingProgress(id, 100.0)).await;

        Ok(())
    }
}
