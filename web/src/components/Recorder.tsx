import React, { useState, useRef, useEffect } from 'react';
import { PlayIcon, PauseIcon, StopIcon, TrashIcon, CloudArrowUpIcon } from '@heroicons/react/24/outline';
import { MicrophoneIcon } from '@heroicons/react/24/solid';
import { startRecording, stopRecording, getRecordingStatus, uploadAudioFile } from '../services/api';
import type { VoiceSession } from '../types';
// import type { RecordingState } from "../types";
import { formatTime, getAudioDuration } from "../utils";

interface RecorderProps {
  onUpload: (session: VoiceSession) => void;
}

const Recorder: React.FC<RecorderProps> = ({ onUpload }) => {
  const [recordingState, setRecordingState] = useState<string | null>();
  const [duration, setDuration] = useState(0);
  const [audioBlob, setAudioBlob] = useState<Blob | null>(null);
  const [audioUrl, setAudioUrl] = useState<string | null>(null);
  const [isPlaying, setIsPlaying] = useState(false);
  const [playbackTime, setPlaybackTime] = useState(0);
  const [totalDuration, setTotalDuration] = useState(0);
  const [isUploading, setIsUploading] = useState(false);
  const [isStoppingRecording, setIsStoppingRecording] = useState(false);
  const [error, setError] = useState<string | null>(null);
  
  const mediaRecorderRef = useRef<MediaRecorder | null>(null);
  const audioRef = useRef<HTMLAudioElement | null>(null);
  const intervalRef = useRef<NodeJS.Timeout | null>(null);
  const chunksRef = useRef<Blob[]>([]);
  const recordingSessionRef = useRef<string | null>(null);

  // Clean up timer on unmount
  useEffect(() => {
    return () => {
      if (intervalRef.current) {
        clearInterval(intervalRef.current);
      }
    };
  }, []);

  const startRecordingHandler = async () => {
    try {
      setError(null);
      
      // Start recording session on backend
      const sessionId = await startRecording();
      recordingSessionRef.current = sessionId;
      
      // Start local recording for preview
      const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
      const mediaRecorder = new MediaRecorder(stream);
      mediaRecorderRef.current = mediaRecorder;
      chunksRef.current = [];

      mediaRecorder.ondataavailable = (event) => {
        if (event.data.size > 0) {
          chunksRef.current.push(event.data);
        }
      };

      mediaRecorder.onstop = () => {
        const audioBlob = new Blob(chunksRef.current, { type: 'audio/wav' });
        setAudioBlob(audioBlob);
        setAudioUrl(URL.createObjectURL(audioBlob));
        
        // Stop all tracks to release the microphone
        stream.getTracks().forEach(track => track.stop());
      };

      mediaRecorder.start();
      setRecordingState('recording');
      setDuration(0);
      
      // Start timer
      intervalRef.current = setInterval(() => {
        setDuration(prev => prev + 1);
      }, 1000);
      
    } catch (error) {
      console.error('Error starting recording:', error);
      setError('Failed to start recording. Please check your microphone permissions.');
    }
  };

  const stopRecordingHandler = async () => {
    if (mediaRecorderRef.current && recordingState === 'recording') {
      try {
        setError(null);
        setIsStoppingRecording(true);
        
        // Stop local recording
        mediaRecorderRef.current.stop();
        
        if (intervalRef.current) {
          clearInterval(intervalRef.current);
          intervalRef.current = null;
        }
        
        // Stop recording session on backend with async processing
        if (recordingSessionRef.current) {
          await stopRecording();
        }
        
        // Set state to stopped after backend processing completes
        setRecordingState('stopped');
      } catch (error) {
        console.error('Error stopping recording:', error);
        setError('Failed to stop recording properly.');
        // Still set to stopped state even if backend fails
        setRecordingState('stopped');
      } finally {
        setIsStoppingRecording(false);
      }
    }
  };

  // Toggle audio playback
  const togglePlayback = () => {
    if (audioRef.current && audioUrl) {
      if (isPlaying) {
        audioRef.current.pause();
        setIsPlaying(false);
      } else {
        audioRef.current.play();
        setIsPlaying(true);
      }
    }
  };

  // Update audio element when audioUrl changes
  useEffect(() => {
    if (audioRef.current && audioUrl) {
      audioRef.current.src = audioUrl;
      audioRef.current.onloadedmetadata = () => {
        if (audioRef.current) {
          setTotalDuration(audioRef.current.duration);
        }
      };
      audioRef.current.ontimeupdate = () => {
        if (audioRef.current) {
          setPlaybackTime(audioRef.current.currentTime);
        }
      };
      audioRef.current.onended = handleAudioEnded;
    }
  }, [audioUrl]);

  // Handle audio events
  const handleAudioEnded = () => {
    setIsPlaying(false);
  };

  // Clear recording
  const clearRecording = () => {
    setRecordingState('idle');
    setDuration(0);
    setAudioBlob(null);
    setAudioUrl(null);
    setIsPlaying(false);
    setPlaybackTime(0);
    setTotalDuration(0);
    setIsStoppingRecording(false);
    setError(null);
    recordingSessionRef.current = null;
    
    if (audioRef.current) {
      audioRef.current.pause();
      audioRef.current.currentTime = 0;
    }
  };

  // Upload recording
  const uploadRecording = async () => {
    if (audioBlob) {
      try {
        setIsUploading(true);
        setError(null);
        
        const audioFile = new File([audioBlob], `recording_${Date.now()}.wav`, {
          type: 'audio/wav'
        });
        
        // Upload and process the audio file
        const session = await uploadAudioFile(audioFile);
        
        // Notify parent component with the processed session
        await onUpload(session);
        
        // Reset recorder state
        clearRecording();
      } catch (error) {
        console.error('Error uploading recording:', error);
        setError('Failed to upload and process recording. Please try again.');
      } finally {
        setIsUploading(false);
      }
    }
  };

  return (
    <div className="bg-white rounded-2xl shadow-lg p-8 max-w-md mx-auto">
      {/* Error Display */}
      {error && (
        <div className="mb-6 p-4 bg-red-50 border border-red-200 rounded-lg">
          <div className="flex items-center">
            <svg className="w-5 h-5 text-red-400 mr-2" fill="currentColor" viewBox="0 0 20 20">
              <path fillRule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zM8.707 7.293a1 1 0 00-1.414 1.414L8.586 10l-1.293 1.293a1 1 0 101.414 1.414L10 11.414l1.293 1.293a1 1 0 001.414-1.414L11.414 10l1.293-1.293a1 1 0 00-1.414-1.414L10 8.586 8.707 7.293z" clipRule="evenodd" />
            </svg>
            <span className="text-red-800 text-sm">{error}</span>
          </div>
        </div>
      )}
      
      {/* Timer Display */}
      <div className="text-center mb-8">
        <div className="text-4xl font-mono font-bold text-gray-800 mb-2">
          {formatTime(duration)}
        </div>
        <div className="text-sm text-gray-500">
          {isStoppingRecording ? (
            <div className="flex items-center justify-center space-x-2">
              <div className="animate-spin rounded-full h-4 w-4 border-b-2 border-blue-600"></div>
              <span>Processing recording...</span>
            </div>
          ) : (
            recordingState === 'recording' ? 'Recording...' : 
            recordingState === 'stopped' ? 'Recording complete' : 'Ready to record'
          )}
        </div>
      </div>

      {/* Audio Preview */}
      {audioUrl && (
        <div className="mb-6">
          <audio ref={audioRef} className="hidden" />
          
          {/* Custom Audio Player */}
          <div className="bg-gray-50 rounded-xl p-4">
            <div className="flex items-center justify-between mb-3">
              <span className="text-sm font-medium text-gray-700">Preview</span>
              <span className="text-xs text-gray-500">
                {formatTime(Math.floor(playbackTime))} / {formatTime(Math.floor(totalDuration))}
              </span>
            </div>
            
            {/* Progress Bar */}
            <div className="relative w-full bg-gray-200 rounded-full h-2 mb-4">
              <div 
                className="bg-indigo-600 h-2 rounded-full transition-all duration-300"
                style={{ width: `${totalDuration > 0 ? (playbackTime / totalDuration) * 100 : 0}%` }}
              />
              <input 
                type="range"
                min="0"
                max={totalDuration || 0}
                value={playbackTime}
                onChange={(e) => {
                  if (audioRef.current) {
                    audioRef.current.currentTime = Number(e.target.value);
                    setPlaybackTime(Number(e.target.value));
                  }
                }}
                className="absolute inset-0 w-full h-2 opacity-0 cursor-pointer"
              />
            </div>
            
            {/* Playback Controls */}
            <div className="flex items-center justify-center space-x-4">
              <button
                onClick={togglePlayback}
                className="flex items-center justify-center w-12 h-12 bg-indigo-600 text-white rounded-full hover:bg-indigo-700 transition-colors"
              >
                {isPlaying ? (
                  <PauseIcon className="w-6 h-6" />
                ) : (
                  <PlayIcon className="w-6 h-6 ml-1" />
                )}
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Recording Controls */}
      <div className="flex items-center justify-center space-x-4">
        {recordingState === 'idle' && (
          <button
            onClick={startRecordingHandler}
            className="flex items-center justify-center w-16 h-16 bg-red-500 text-white rounded-full hover:bg-red-600 transition-all duration-200 transform hover:scale-105 shadow-lg"
          >
            <MicrophoneIcon className="w-8 h-8" />
          </button>
        )}
        
        {recordingState === 'recording' && (
          <button
            onClick={stopRecordingHandler}
            disabled={isStoppingRecording}
            className={`flex items-center justify-center w-16 h-16 text-white rounded-full transition-all duration-200 transform shadow-lg ${
              isStoppingRecording 
                ? 'bg-blue-500 cursor-not-allowed' 
                : 'bg-gray-600 hover:bg-gray-700 hover:scale-105'
            }`}
          >
            {isStoppingRecording ? (
              <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-white"></div>
            ) : (
              <StopIcon className="w-8 h-8" />
            )}
          </button>
        )}
        
        {recordingState === 'stopped' && (
          <>
            <button
              onClick={clearRecording}
              className="flex items-center justify-center w-12 h-12 bg-gray-500 text-white rounded-full hover:bg-gray-600 transition-colors"
            >
              <TrashIcon className="w-6 h-6" />
            </button>
            
            <button
              onClick={startRecordingHandler}
              className="flex items-center justify-center w-16 h-16 bg-red-500 text-white rounded-full hover:bg-red-600 transition-all duration-200 transform hover:scale-105 shadow-lg"
            >
              <MicrophoneIcon className="w-8 h-8" />
            </button>
            
            <button
              onClick={uploadRecording}
              disabled={isUploading}
              className="flex items-center justify-center w-12 h-12 bg-green-500 text-white rounded-full hover:bg-green-600 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {isUploading ? (
                <div className="animate-spin rounded-full h-6 w-6 border-b-2 border-white"></div>
              ) : (
                <CloudArrowUpIcon className="w-6 h-6" />
              )}
            </button>
          </>
        )}
      </div>
      
      {/* Action Labels */}
      <div className="text-center mt-4">
        {recordingState === 'idle' && (
          <p className="text-sm text-gray-600">Tap to start recording</p>
        )}
        {recordingState === 'recording' && (
          <p className="text-sm text-gray-600">
            {isStoppingRecording ? 'Stopping recording...' : 'Tap to stop recording'}
          </p>
        )}
        {recordingState === 'stopped' && (
          <div className="flex justify-center space-x-8 text-xs text-gray-500">
            <span>Clear</span>
            <span>Record Again</span>
            <span>{isUploading ? 'Processing...' : 'Process & Save'}</span>
          </div>
        )}
      </div>
    </div>
  );
};

export default Recorder;