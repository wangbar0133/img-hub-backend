# Image Hub Backend API Documentation

## Base URL
```
http://localhost:8000
```

## Endpoints

### 1. Health Check
**GET** `/`

Returns a simple health check message.

**Response:**
```
"Hello, world!"
```

---

### 2. Get All Albums
**GET** `/api/albums`

Retrieves all photo albums from the database.

---

### 3. Get Featured Albums
**GET** `/api/featured_albums`

Retrieves only albums marked as featured (featured = true).

**Response:**
```json
{
  "success": true,
  "msg": null,
  "albums": [
    {
      "id": "string",
      "title": "string", 
      "cover": "string",
      "category": "string",
      "shot_time": "2024-01-01T00:00:00Z",
      "updata_time": "2024-01-01T00:00:00Z",
      "featured": false,
      "hidden": false,
      "photos": [
        {
          "src": "string",
          "detail": "string", 
          "medium": "string",
          "thumbnail": "string",
          "info": {
            "width": 1920,
            "height": 1080,
            "format": "JPEG",
            "file_size": 1024000,
            "created_at": "2024-01-01T00:00:00Z",
            "camera_make": "Canon",
            "camera_model": "EOS R5",
            "lens_model": "RF 24-70mm F2.8 L IS USM",
            "focal_length": 50.0,
            "aperture": 2.8,
            "exposure_time": "1/125",
            "iso": 100,
            "flash": "No Flash",
            "white_balance": "Auto"
          }
        }
      ]
    }
  ]
}
```

**Error Response:**
```json
{
  "success": false,
  "msg": "Failed to retrieve albums",
  "albums": []
}
```

---

### 4. Get Album by ID
**GET** `/api/album/{id}`

Retrieves a specific album by its ID.

**Parameters:**
- `id` (string, path): Album ID

**Response:**
```json
{
  "success": true,
  "msg": null,
  "album": {
    "id": "string",
    "title": "string",
    "cover": "string", 
    "category": "string",
    "shot_time": "2024-01-01T00:00:00Z",
    "updata_time": "2024-01-01T00:00:00Z",
    "featured": false,
    "hidden": false,
    "photos": [...]
  }
}
```

**Error Response:**
```json
{
  "success": false,
  "msg": "Failed to retrieve albums",
  "album": null
}
```

---

### 5. Upload Images (Async)
**POST** `/api/upload`

Uploads multiple images and creates compressed versions (thumbnail, detail, original) asynchronously. Returns immediately with a task ID for tracking progress.

**Content-Type:** `multipart/form-data`

**Form Fields:**
- `images` (file[], required): Unlimited number of image files
- `id` (string, optional): Album ID (defaults to auto-generated UUID)
- `title` (string, optional): Album title (defaults to "Untitled Album")
- `category` (string, optional): Album category (defaults to "cosplay")
- `featured` (string, optional): Whether album is featured ("true"/"false", defaults to "false")
- `hidden` (string, optional): Whether album is hidden ("true"/"false", defaults to "false")

**Response:**
```json
{
  "success": true,
  "task_id": "upload-12345678-1234-1234-1234-123456789abc",
  "msg": "Upload task created. Processing 40 files in background."
}
```

**Error Response:**
```json
{
  "success": false,
  "task_id": "upload-12345678-1234-1234-1234-123456789abc",
  "msg": "Failed to parse upload data"
}
```

**Image Processing:**
- **Source**: Original filename (src)
- **Detail**: Compressed to max 1920x1080, 100% quality
- **Medium**: Compressed to max 800x600, 100% quality
- **Thumbnail**: Compressed to max 300x300, 100% quality
- All images maintain aspect ratio during compression
- Processing happens in background, use `/api/upload-status/{task_id}` to track progress

---

### 6. Get Upload Status
**GET** `/api/upload-status/{task_id}`

Retrieves the current status of an upload task.

**Parameters:**
- `task_id` (string, path): Upload task ID returned from `/api/upload`

**Response:**
```json
{
  "success": true,
  "task": {
    "task_id": "upload-12345678-1234-1234-1234-123456789abc",
    "status": "Processing",
    "total_files": 40,
    "processed_files": 25,
    "failed_files": 2,
    "album_id": "album-id-123",
    "error_message": null,
    "created_at": "2025-09-27T08:30:00Z",
    "completed_at": null
  },
  "msg": null
}
```

**Status Values:**
- `Processing`: Task is currently being processed
- `Completed`: Task completed successfully
- `Failed`: Task failed due to an error

**Error Response:**
```json
{
  "success": false,
  "task": null,
  "msg": "Task not found"
}
```

**Usage Flow:**
1. Call `/api/upload` to start upload (get task_id)
2. Poll `/api/upload-status/{task_id}` until status is `Completed` or `Failed`
3. Check `processed_files` and `failed_files` for progress details

---

### 7. Set Album Cover
**PUT** `/api/album/{album_id}/cover`

Sets the cover image for a specific album.

**Parameters:**
- `album_id` (string, path): Album ID

**Content-Type:** `application/json`

**Request Body:**
```json
{
  "cover": "image1_medium.jpg"
}
```

**Response:**
```json
{
  "success": true,
  "msg": "Album cover updated to: image1_medium.jpg"
}
```

**Error Response:**
```json
{
  "success": false,
  "msg": "Album not found"
}
```

**Error Cases:**
- Album not found
- Cover file does not exist in the album
- Missing or invalid cover filename
- Database error

---

### 8. Delete Album
**DELETE** `/api/album/{album_id}`

Deletes a specific album and all its associated photos from the database and file system.

**Parameters:**
- `album_id` (string, path): Album ID

**Response:**
```json
{
  "success": true,
  "msg": "Album successfully deleted"
}
```

**Error Response:**
```json
{
  "success": false,
  "msg": "Album not found"
}
```

**Error Cases:**
- Album not found
- Database error
- File system error during photo deletion

**Note:** This operation permanently deletes:
- Album record from database
- All photo files (src, detail, medium, thumbnail) from file system
- Cannot be undone

---

### 9. Static File Server
**GET** `/public/{filename}`

Serves uploaded images and generated thumbnails.

**Parameters:**
- `filename` (string, path): Image filename

**Example:**
```
GET /public/image1.jpg
GET /public/image1_thumbnail.jpg
GET /public/image1_detail.jpg
GET /public/image1_medium.jpg
```

---

## Data Models

### Album
```json
{
  "id": "string",
  "title": "string",
  "cover": "string",
  "category": "string",
  "shot_time": "ISO 8601 datetime",
  "updata_time": "ISO 8601 datetime",
  "featured": "boolean",
  "hidden": "boolean",
  "photos": "Photo[]"
}
```

### Photo
```json
{
  "src": "string",
  "detail": "string",
  "medium": "string", 
  "thumbnail": "string",
  "info": "ImageInfo"
}
```

### ImageInfo
```json
{
  "width": "number",
  "height": "number",
  "format": "string",
  "file_size": "number",
  "created_at": "ISO 8601 datetime|null",
  "camera_make": "string|null",
  "camera_model": "string|null",
  "lens_model": "string|null",
  "focal_length": "number|null",
  "aperture": "number|null",
  "exposure_time": "string|null",
  "iso": "number|null",
  "flash": "string|null",
  "white_balance": "string|null"
}
```

### UploadTask
```json
{
  "task_id": "string",
  "status": "Processing|Completed|Failed",
  "total_files": "number",
  "processed_files": "number",
  "failed_files": "number",
  "album_id": "string|null",
  "error_message": "string|null",
  "created_at": "ISO 8601 datetime",
  "completed_at": "ISO 8601 datetime|null"
}
```

---

## Error Handling

All endpoints return JSON responses with the following structure:

**Success:**
```json
{
  "success": true,
  "msg": "string|null",
  "data": "..."
}
```

**Error:**
```json
{
  "success": false,
  "msg": "Error description",
  "data": "null or empty array/object"
}
```

---

## Configuration

- **Upload Directory**: `static/` (configurable via UPLOAD_DIR env var)
- **Max File Upload**: Unlimited (async processing)
- **Supported Formats**: JPEG, PNG (determined by image crate)
- **Database**: MongoDB on `localhost:27017` (configurable via DATABASE_URL env var)
- **Database Name**: `img-hub`
- **Collections**: `albums`, `upload_tasks`
- **Processing**: Async background processing with progress tracking
