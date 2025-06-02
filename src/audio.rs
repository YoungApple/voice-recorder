// src/audio.rs
use anyhow::Result;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, StreamConfig};
use hound::{WavSpec, WavWriter};
use std::sync::{Arc, Mutex};
use std::time::Instant;

pub struct VoiceRecorder {
    device: Arc<Mutex<Device>>,
    config: Arc<StreamConfig>,
    current_session: Arc<Mutex<Option<crate::storage::VoiceSession>>>,
    recording_start: Arc<Mutex<Option<Instant>>>,
    stream: Arc<Mutex<Option<cpal::Stream>>>,
    is_recording: Arc<Mutex<bool>>,
}

// Implement Send for VoiceRecorder
unsafe impl Send for VoiceRecorder {}
unsafe impl Sync for VoiceRecorder {}

impl VoiceRecorder {
    pub async fn new() -> Result<Self> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or_else(|| anyhow::anyhow!("No input device available"))?;
        
        let config = device.default_input_config()?;
        
        println!("Using audio device: {}", device.name()?);
        println!("Default input config: {:?}", config);
        
        Ok(Self {
            device: Arc::new(Mutex::new(device)),
            config: Arc::new(config.into()),
            current_session: Arc::new(Mutex::new(None)),
            recording_start: Arc::new(Mutex::new(None)),
            stream: Arc::new(Mutex::new(None)),
            is_recording: Arc::new(Mutex::new(false)),
        })
    }
    
    pub async fn start_recording(&mut self) -> Result<()> {
        // Check if already recording
        if *self.is_recording.lock().unwrap() {
            println!("Recording is already in progress");
            return Ok(());
        }
        
        let session = crate::storage::create_new_session();
        println!("Created session: {}", session.id);
        
        let spec = WavSpec {
            channels: self.config.channels,
            sample_rate: self.config.sample_rate.0,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        
        let writer = WavWriter::create(&session.audio_file_path, spec)?;
        let writer = Arc::new(Mutex::new(Some(writer)));
        let writer_clone = writer.clone();
        
        let stream = {
            let device = self.device.lock().unwrap();
            let config = self.config.clone();
            device.build_input_stream(
                &config,
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    if let Ok(mut guard) = writer_clone.lock() {
                        if let Some(writer) = guard.as_mut() {
                            for &sample in data {
                                let sample_i16 = (sample * 32767.0) as i16;
                                let _ = writer.write_sample(sample_i16);
                            }
                        }
                    }
                },
                |err| eprintln!("Error in audio stream: {}", err),
                None,
            )?
        };
        
        stream.play()?;
        
        // Store stream properly
        {
            let mut stream_guard = self.stream.lock().unwrap();
            *stream_guard = Some(stream);
        }
        
        // Update session and recording state
        {
            let mut session_guard = self.current_session.lock().unwrap();
            *session_guard = Some(session);
        }
        {
            let mut start_guard = self.recording_start.lock().unwrap();
            *start_guard = Some(Instant::now());
        }
        {
            let mut recording_guard = self.is_recording.lock().unwrap();
            *recording_guard = true;
        }
        
        println!("Recording started. Press 'e' to stop recording.");
        
        Ok(())
    }
    
    pub async fn stop_recording(&mut self) -> Result<()> {
        // Check recording status
        if !*self.is_recording.lock().unwrap() {
            println!("No recording in progress");
            return Ok(());
        }

        // Stop and drop the stream
        {
            let mut stream_guard = self.stream.lock().unwrap();
            if let Some(stream) = stream_guard.take() {
                drop(stream);
            }
        }

        // Set recording status to false
        {
            let mut recording_guard = self.is_recording.lock().unwrap();
            *recording_guard = false;
        }
        
        // Get session and process it
        let session = {
            let mut session_guard = self.current_session.lock().unwrap();
            session_guard.take()
        };

        if let Some(mut session) = session {
            // Get recording start time and calculate duration
            {
                let mut start_guard = self.recording_start.lock().unwrap();
                if let Some(start_time) = start_guard.take() {
                    session.duration_ms = start_time.elapsed().as_millis() as u64;
                }
            }
            
            println!("🔄 Processing audio...  #[rexrex]");
            
            // Process the audio file
            if let Ok(transcript) = crate::ai::transcribe_audio(&session.audio_file_path).await {
                session.transcript = Some(transcript.clone());
                println!("📝 Transcript: {}", transcript);
                
                // Analyze the transcript
                if let Ok(analysis) = crate::ai::analyze_transcript(&transcript).await {
                    session.analysis = Some(analysis.clone());
                    
                    // Generate title from analysis
                    if let Some(first_idea) = analysis.ideas.first() {
                        session.title = first_idea.clone();
                    } else if !analysis.tasks.is_empty() {
                        session.title = analysis.tasks[0].title.clone();
                    } else {
                        session.title = "Voice Note".to_string();
                    }
                    
                    // Display analysis results
                    println!("\n📊 Analysis Results:");
                    println!("� Ideas: {}", analysis.ideas.len());
                    for idea in &analysis.ideas {
                        println!("  • {}", idea);
                    }
                    
                    println!("✅ Tasks: {}", analysis.tasks.len());
                    for task in &analysis.tasks {
                        println!("  • {} (Priority: {:?})", task.title, task.priority);
                    }
                    
                    println!("📝 Notes: {}", analysis.structured_notes.len());
                    for note in &analysis.structured_notes {
                        println!("  • {} (Type: {:?})", note.title, note.note_type);
                    }
                    
                    println!("📋 Summary: {}", analysis.summary);
                }
            }
            
            // Save session
            let analysis_to_save = session.analysis.take();
            crate::storage::save_session(&mut session, analysis_to_save).await?;
            println!("💾 Session saved: {}", session.id);
        }
        
        Ok(())
    }

    pub async fn play_audio_file(&self, file_path: &str) -> Result<()> {
        let file = std::fs::File::open(file_path)?;
        let mut reader = hound::WavReader::new(file)?;
        let spec = reader.spec();

        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or_else(|| anyhow::anyhow!("No output device available"))?;

        let config: cpal::StreamConfig = device.default_output_config()?.into();

        let samples = reader
            .samples::<i16>()
            .map(|s| s.unwrap() as f32 / 32768.0)
            .collect::<Vec<f32>>();
        let mut samples_iter = samples.into_iter();

        let stream = device.build_output_stream(
            &config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                for sample in data {
                    *sample = samples_iter.next().unwrap_or(0.0);
                }
            },
            |err| eprintln!("Error in audio playback stream: {}", err),
            None,
        )?;

        stream.play()?;

        // Keep the stream alive until playback is finished
        // This is a simple way; for real applications, you might want a more robust mechanism
        // to know when playback is truly done (e.g., by checking if samples_iter is exhausted).
        tokio::time::sleep(std::time::Duration::from_secs(
            (reader.len() as f32 / spec.sample_rate as f32 / spec.channels as f32) as u64 + 1,
        ))
        .await;

        Ok(())
    }

    pub fn is_recording(&self) -> bool {
        *self.is_recording.lock().unwrap()
    }
}