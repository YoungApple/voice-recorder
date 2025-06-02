use std::future::Future;
use tokio::sync::mpsc;
use std::sync::{Arc, Mutex};

use rdev::{listen, Event, EventType, Key};
use lazy_static::lazy_static;
use tokio::sync::Mutex as AsyncMutex;


enum KeyEvent {
    StartRecord,
    EndRecord,
    Quit,
}



lazy_static! {
    static ref GLOBAL_SENDER: Mutex<Option<tokio::sync::mpsc::Sender<KeyEvent>>> = Mutex::new(None);
}

pub struct KeyboardHandler {
    recorder: Arc<AsyncMutex<crate::audio::VoiceRecorder>>,
}

impl KeyboardHandler {
    pub fn new(recorder: Arc<AsyncMutex<crate::audio::VoiceRecorder>>) -> Self {
        Self {
            recorder,
        }
    }
    
    pub fn start_listening(&mut self) -> anyhow::Result<impl Future<Output = ()> + Send> {
        let (tx, mut rx) = mpsc::channel(100); // Use tokio::sync::mpsc channel with a buffer size
        *GLOBAL_SENDER.lock().unwrap() = Some(tx);
        
        let recorder = self.recorder.clone();
        
        // Spawn thread for keyboard event listening
        std::thread::spawn(move || {
            if let Err(error) = listen(Self::handle_event) {
                println!("Error listening to keyboard events: {:?}", error);
            }
        });

        // Return future that will process events
        Ok(async move {
            loop {
                tokio::select! {
                    Some(key_event) = rx.recv() => {
                        match key_event {
                            KeyEvent::StartRecord => {
                                let result = {
                                    let mut guard = recorder.lock().await;
                                    guard.start_recording().await
                                };
                                if let Err(e) = result {
                                    println!("Failed to start recording: {:?}", e);
                                }
                            }
                            KeyEvent::EndRecord => {
                                let result = {
                                    let mut guard = recorder.lock().await;
                                    guard.stop_recording().await
                                };
                                if let Err(e) = result {
                                    println!("Failed to stop recording: {:?}", e);
                                }
                            }
                            KeyEvent::Quit => {
                                println!("Quit event received. Exiting.");
                                break;
                            }
                        }
                    }
                    _ = tokio::signal::ctrl_c() => {
                        println!("Ctrl-C received. Exiting keyboard listener.");
                        break;
                    }
                }
            }
        })
    }

    fn handle_event(event: Event) {
        let key_event_option = match event.event_type {
            EventType::KeyPress(key) => match key {
                Key::KeyR => Some(KeyEvent::StartRecord),
                Key::KeyE => Some(KeyEvent::EndRecord),
                Key::KeyQ => Some(KeyEvent::Quit),
                _ => None,
            },
            EventType::KeyRelease(_) => None, // Ignore key release events for now
            _ => None, // Ignore other event types
        };

        if let Some(key_event) = key_event_option {
            if let Some(tx) = GLOBAL_SENDER.lock().unwrap().as_ref() {
                // Use a blocking send here as we are in a non-async context (rdev listener thread)
                if let Err(e) = tx.blocking_send(key_event) {
                    eprintln!("Failed to send key event: {:?}", e);
                }
            }
        }
    }
}