# 快速录音功能流程设计

## 概述
本文档概述了快速录音功能的实现流程，该功能允许用户快速录制新笔记，并在前端和后端进行音频处理。

## 当前架构分析

### 前端组件
- **录音组件** (`/web/src/components/Recorder.tsx`): 使用MediaRecorder API处理音频录制
- **主页** (`/web/src/pages/Home.tsx`): 主界面，带有新录音的浮动操作按钮
- **API服务** (`/web/src/services/api.ts`): 处理与后端的通信

### 后端组件
- **音频模块** (`/src/audio.rs`): VoiceRecorder结构体用于音频捕获和处理
- **Web模块** (`/src/web.rs`): 录音控制的HTTP API端点
- **存储模块** (`/src/storage.rs`): 会话和分析数据持久化
- **AI模块** (`/src/ai.rs`): 音频转录和分析

## 快速录音流程设计

### 1. Frontend Audio Recording Flow

```
User clicks "New Recording" button
    ↓
Open recording modal with Recorder component
    ↓
Request microphone permissions
    ↓
Start MediaRecorder with audio stream
    ↓
Show recording UI with timer and controls
    ↓
User stops recording
    ↓
Generate audio blob (WAV format)
    ↓
Send audio file to backend for processing
    ↓
Show processing indicator
    ↓
Receive processed session data
    ↓
Update notes list and close modal
```

### 2. Backend Processing Flow

```
Receive audio file upload
    ↓
Create new VoiceSession with unique ID
    ↓
Save audio file to storage directory
    ↓
Transcribe audio using AI service (Ollama)
    ↓
Analyze transcript for:
    - Ideas extraction
    - Tasks identification
    - Structured notes creation
    - Summary generation
    ↓
Save session with analysis results
    ↓
Return session data to frontend
```

### 3. API Endpoints

#### Current Endpoints
- `POST /api/record/start` - Start recording session
- `POST /api/record/stop` - Stop recording session
- `GET /api/record/status` - Get recording status
- `GET /api/sessions` - List all sessions
- `GET /api/sessions/:id` - Get specific session

#### Required New Endpoint
- `POST /api/sessions/upload` - Upload audio file and create session

### 4. Implementation Steps

#### Step 1: Create Audio Upload Endpoint
```rust
// In src/web.rs
async fn upload_audio_handler(
    State(recorder): State<Arc<AsyncMutex<VoiceRecorder>>>,
    mut multipart: Multipart,
) -> Result<Json<ApiResponse<VoiceSession>>, StatusCode> {
    // Handle multipart file upload
    // Create new session
    // Process audio file
    // Return session data
}
```

#### Step 2: Update Frontend API Service
```typescript
// In web/src/services/api.ts
export async function uploadAudioFile(audioFile: File): Promise<VoiceSession> {
    const formData = new FormData();
    formData.append('audio', audioFile);
    
    const response = await apiClient.post<ApiResponse<VoiceSession>>(
        '/sessions/upload',
        formData,
        {
            headers: {
                'Content-Type': 'multipart/form-data',
            },
        }
    );
    
    return response.data.data;
}
```

#### Step 3: Update Recorder Component
```typescript
// In web/src/components/Recorder.tsx
const uploadRecording = async () => {
    if (audioBlob) {
        try {
            setIsUploading(true);
            
            const audioFile = new File([audioBlob], `recording_${Date.now()}.wav`, {
                type: 'audio/wav'
            });
            
            const session = await uploadAudioFile(audioFile);
            
            // Notify parent component of successful upload
            onUpload(session);
            
            // Reset recorder state
            clearRecording();
        } catch (error) {
            setError('Failed to upload recording');
        } finally {
            setIsUploading(false);
        }
    }
};
```

#### Step 4: Update Home Page Integration
```typescript
// In web/src/pages/Home.tsx
const handleUpload = async (session: VoiceSession) => {
    try {
        // Add new session to the list
        setSessions(prev => [session, ...prev]);
        
        // Close recording modal
        setIsRecording(false);
        
        // Show success message
        console.log('Recording processed successfully:', session.title);
    } catch (error) {
        setError('Failed to process recording');
    }
};
```

### 5. Enhanced Features

#### Real-time Processing Feedback
- Show processing stages: "Uploading...", "Transcribing...", "Analyzing..."
- Progress indicators for each stage
- Estimated time remaining

#### Error Handling
- Network connectivity issues
- Microphone permission denied
- Audio processing failures
- File size limitations

#### Optimizations
- Audio compression before upload
- Chunked upload for large files
- Background processing queue
- Caching of processed results

### 6. User Experience Flow

1. **Quick Access**: Floating action button always visible
2. **Instant Feedback**: Visual recording indicators
3. **Progress Tracking**: Clear processing stages
4. **Error Recovery**: Retry mechanisms and clear error messages
5. **Seamless Integration**: Automatic list updates after processing

### 7. Technical Considerations

#### Audio Quality
- Sample rate: 44.1kHz or 48kHz
- Bit depth: 16-bit
- Format: WAV for compatibility
- Compression: Optional MP3 encoding for smaller files

#### Performance
- Lazy loading of audio files
- Efficient memory management
- Background processing
- Responsive UI during processing

#### Security
- File type validation
- Size limitations
- Secure file storage
- User permission management

### 8. Testing Strategy

#### Frontend Testing
- MediaRecorder API compatibility
- File upload functionality
- UI state management
- Error handling scenarios

#### Backend Testing
- Audio file processing
- AI service integration
- Database operations
- API endpoint validation

#### Integration Testing
- End-to-end recording flow
- Cross-browser compatibility
- Mobile device testing
- Network failure scenarios

This design provides a comprehensive foundation for implementing the quick record feature with proper separation of concerns and robust error handling.