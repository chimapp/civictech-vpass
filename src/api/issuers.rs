use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    routing::{get, post},
    Form, Json, Router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api::middleware::session::AppState;
use crate::models::issuer::{CardIssuer, CreateIssuerData};
use crate::services::youtube_channel;

#[derive(Debug)]
pub enum IssuersError {
    DatabaseError(sqlx::Error),
    NotFound,
    ValidationError(String),
    YouTubeApiError(youtube_channel::YouTubeChannelError),
}

impl IntoResponse for IssuersError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            IssuersError::DatabaseError(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Database error: {}", e),
            ),
            IssuersError::NotFound => (StatusCode::NOT_FOUND, "Issuer not found".to_string()),
            IssuersError::ValidationError(msg) => (StatusCode::BAD_REQUEST, msg),
            IssuersError::YouTubeApiError(e) => {
                (StatusCode::BAD_REQUEST, format!("YouTube API error: {}", e))
            }
        };

        (status, message).into_response()
    }
}

/// List all issuers
async fn list_issuers(State(state): State<AppState>) -> Result<Html<String>, IssuersError> {
    let issuers = CardIssuer::list_active(&state.pool)
        .await
        .map_err(IssuersError::DatabaseError)?;

    let issuers_html: String = issuers
        .iter()
        .map(|issuer| {
            format!(
                r#"<div class="issuer-card">
                    <div class="issuer-header">
                        <h3>{}</h3>
                        <span class="status {}">{}</span>
                    </div>
                    <div class="info-grid">
                        <div class="info-item">
                            <span class="label">Channel ID</span>
                            <span class="value">{}</span>
                        </div>
                        <div class="info-item">
                            <span class="label">Handle</span>
                            <span class="value">{}</span>
                        </div>
                        <div class="info-item">
                            <span class="label">Verification Video</span>
                            <span class="value">{}</span>
                        </div>
                        <div class="info-item">
                            <span class="label">Default Label</span>
                            <span class="value">{}</span>
                        </div>
                    </div>
                    <div class="actions">
                        <a href="/channels/{}/claim" class="btn btn-primary">Claim Card</a>
                        <a href="/issuers/{}/edit" class="btn btn-secondary">Edit</a>
                        <form action="/issuers/{}/toggle" method="POST" style="display: inline;">
                            <button type="submit" class="btn btn-warning">{}</button>
                        </form>
                    </div>
                </div>"#,
                issuer.channel_name,
                if issuer.is_active {
                    "active"
                } else {
                    "inactive"
                },
                if issuer.is_active {
                    "Active"
                } else {
                    "Inactive"
                },
                issuer.youtube_channel_id,
                issuer.channel_handle.as_deref().unwrap_or("N/A"),
                issuer.verification_video_id,
                issuer.default_membership_label,
                issuer.id,
                issuer.id,
                issuer.id,
                if issuer.is_active {
                    "Deactivate"
                } else {
                    "Activate"
                }
            )
        })
        .collect();

    let html = format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <title>Card Issuers - VPass</title>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <style>
        * {{ margin: 0; padding: 0; box-sizing: border-box; }}
        body {{
            font-family: 'Helvetica Neue', Arial, sans-serif;
            background: #E8E6E0;
            color: #000;
            min-height: 100vh;
            padding: 20px;
        }}
        .container {{
            max-width: 1000px;
            margin: 40px auto;
        }}
        .header {{
            display: flex;
            justify-content: space-between;
            align-items: center;
            margin-bottom: 60px;
        }}
        h1 {{
            font-size: 48px;
            font-weight: 300;
            letter-spacing: -1px;
            color: #1E3A5F;
        }}
        .btn {{
            padding: 16px 24px;
            text-decoration: none;
            font-size: 14px;
            font-weight: 600;
            text-transform: uppercase;
            letter-spacing: 1px;
            cursor: pointer;
            transition: all 0.2s;
            box-shadow: 0 2px 4px rgba(0,0,0,0.15);
            border: none;
            display: inline-block;
        }}
        .btn-primary {{
            background: #B8915F;
            color: #000;
        }}
        .btn-primary:hover {{
            background: #1E3A5F;
            color: #fff;
        }}
        .btn-secondary {{
            background: #F5F3ED;
            color: #000;
        }}
        .btn-secondary:hover {{
            background: #1E3A5F;
            color: #fff;
        }}
        .btn-warning {{
            background: #FF5722;
            color: #fff;
        }}
        .btn-warning:hover {{
            background: #D84315;
        }}
        .issuers {{
            display: flex;
            flex-direction: column;
            gap: 2px;
        }}
        .issuer-card {{
            background: #F5F3ED;
            padding: 32px;
            box-shadow: 0 2px 4px rgba(0,0,0,0.08);
        }}
        .issuer-header {{
            display: flex;
            justify-content: space-between;
            align-items: center;
            margin-bottom: 24px;
        }}
        .issuer-header h3 {{
            font-size: 24px;
            font-weight: 500;
            color: #1E3A5F;
        }}
        .status {{
            padding: 6px 12px;
            font-size: 10px;
            font-weight: 600;
            text-transform: uppercase;
            letter-spacing: 1px;
        }}
        .status.active {{
            background: #B8915F;
            color: #000;
        }}
        .status.inactive {{
            background: #8C8C88;
            color: #fff;
        }}
        .info-grid {{
            display: grid;
            grid-template-columns: repeat(2, 1fr);
            gap: 16px;
            margin-bottom: 24px;
        }}
        .info-item {{
            display: flex;
            flex-direction: column;
            gap: 4px;
        }}
        .label {{
            font-size: 10px;
            text-transform: uppercase;
            letter-spacing: 1px;
            color: #999;
        }}
        .value {{
            font-size: 13px;
            font-family: 'Courier New', monospace;
            color: #000;
            word-break: break-all;
        }}
        .actions {{
            display: flex;
            gap: 8px;
        }}
        .empty {{
            text-align: center;
            padding: 80px 20px;
            background: #F5F3ED;
            box-shadow: 0 2px 4px rgba(0,0,0,0.08);
        }}
        .empty p {{
            font-size: 16px;
            color: #666;
            margin-bottom: 32px;
        }}
        @media (max-width: 600px) {{
            .info-grid {{
                grid-template-columns: 1fr;
            }}
            .actions {{
                flex-direction: column;
            }}
        }}
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <h1>Card Issuers</h1>
            <a href="/issuers/new" class="btn btn-primary">Create Issuer</a>
        </div>
        {}
    </div>
</body>
</html>"#,
        if issuers.is_empty() {
            r#"<div class="empty"><p>No issuers yet.</p><a href="/issuers/new" class="btn btn-primary">Create First Issuer</a></div>"#.to_string()
        } else {
            format!(r#"<div class="issuers">{}</div>"#, issuers_html)
        }
    );

    Ok(Html(html))
}

/// Show create form
async fn new_issuer_form() -> Html<String> {
    Html(
        r#"<!DOCTYPE html>
<html>
<head>
    <title>Create Issuer - VPass</title>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <style>
        * { margin: 0; padding: 0; box-sizing: border-box; }
        body {
            font-family: 'Helvetica Neue', Arial, sans-serif;
            background: #E8E6E0;
            color: #000;
            min-height: 100vh;
            padding: 20px;
        }
        .container {
            max-width: 700px;
            margin: 40px auto;
        }
        .back {
            display: inline-block;
            color: #666;
            text-decoration: none;
            font-size: 13px;
            margin-bottom: 40px;
            transition: color 0.2s;
        }
        .back:hover { color: #1E3A5F; }
        .back::before { content: '← '; }
        h1 {
            font-size: 48px;
            font-weight: 300;
            letter-spacing: -1px;
            color: #1E3A5F;
            margin-bottom: 8px;
        }
        .subtitle {
            font-size: 14px;
            text-transform: uppercase;
            letter-spacing: 2px;
            color: #666;
            margin-bottom: 40px;
        }
        form {
            background: #F5F3ED;
            padding: 40px;
            box-shadow: 0 2px 4px rgba(0,0,0,0.08);
        }
        .form-group {
            margin-bottom: 32px;
        }
        label {
            display: block;
            font-size: 11px;
            font-weight: 600;
            text-transform: uppercase;
            letter-spacing: 1px;
            color: #666;
            margin-bottom: 12px;
        }
        label .required {
            color: #FF5722;
        }
        input, textarea {
            width: 100%;
            padding: 16px;
            background: #fff;
            border: 1px solid #CCC;
            color: #000;
            font-size: 14px;
            font-family: 'Courier New', monospace;
            transition: border-color 0.2s;
        }
        input:focus, textarea:focus {
            outline: none;
            border-color: #1E3A5F;
        }
        input::placeholder, textarea::placeholder {
            color: #999;
        }
        .help-text {
            font-size: 12px;
            color: #666;
            margin-top: 8px;
        }
        button {
            width: 100%;
            padding: 20px;
            background: #B8915F;
            color: #000;
            border: none;
            font-size: 14px;
            font-weight: 600;
            text-transform: uppercase;
            letter-spacing: 1px;
            cursor: pointer;
            transition: all 0.2s;
            box-shadow: 0 2px 4px rgba(0,0,0,0.15);
        }
        button:hover {
            background: #1E3A5F;
            color: #fff;
        }
        button:disabled {
            background: #CCC;
            cursor: not-allowed;
        }
        .autofill-section {
            background: #E8E6E0;
            padding: 32px;
            margin-bottom: 40px;
            border-left: 3px solid #B8915F;
        }
        .autofill-section h2 {
            font-size: 14px;
            font-weight: 600;
            text-transform: uppercase;
            letter-spacing: 1px;
            color: #B8915F;
            margin-bottom: 20px;
        }
        .autofill-input-group {
            display: flex;
            gap: 8px;
        }
        .autofill-input-group input {
            flex: 1;
        }
        .autofill-input-group button {
            width: auto;
            padding: 16px 32px;
            white-space: nowrap;
        }
        .error-message {
            background: #FF5722;
            color: #fff;
            padding: 16px;
            margin-top: 16px;
            font-size: 13px;
            display: none;
        }
        .loading {
            opacity: 0.5;
            pointer-events: none;
        }
    </style>
</head>
<body>
    <div class="container">
        <a href="/issuers" class="back">Back to Issuers</a>
        <h1>Create Issuer</h1>
        <p class="subtitle">Register a new card issuer</p>

        <form action="/issuers" method="POST">
            <div class="autofill-section">
                <h2>Quick Fill from YouTube</h2>
                <div class="autofill-input-group">
                    <input type="text" id="channel_url" placeholder="https://www.youtube.com/@Dokibird">
                    <button type="button" id="autofill-btn">Auto-Fill</button>
                </div>
                <div class="error-message" id="error-message"></div>
            </div>

            <div class="form-group">
                <label for="youtube_channel_id">YouTube Channel ID <span class="required">*</span></label>
                <input type="text" name="youtube_channel_id" id="youtube_channel_id" required
                       placeholder="UCxxxxxxxxxxxxxxxxxxx">
                <p class="help-text">The unique identifier for the YouTube channel</p>
            </div>

            <div class="form-group">
                <label for="channel_name">Channel Name <span class="required">*</span></label>
                <input type="text" name="channel_name" id="channel_name" required
                       placeholder="My Awesome Channel">
            </div>

            <div class="form-group">
                <label for="channel_handle">Channel Handle</label>
                <input type="text" name="channel_handle" id="channel_handle"
                       placeholder="@myawesomechannel">
                <p class="help-text">Optional: The @handle for the channel</p>
            </div>

            <div class="form-group">
                <label for="verification_video_id">Verification Video ID <span class="required">*</span></label>
                <input type="text" name="verification_video_id" id="verification_video_id" required
                       placeholder="dQw4w9WgXcQ">
                <p class="help-text">The video ID where members will post verification comments</p>
            </div>

            <div class="form-group">
                <label for="default_membership_label">Default Membership Label <span class="required">*</span></label>
                <input type="text" name="default_membership_label" id="default_membership_label" required
                       placeholder="Member" value="Member">
                <p class="help-text">The label shown on issued cards (e.g., "Member", "Subscriber")</p>
            </div>

            <button type="submit">Create Issuer</button>
        </form>
    </div>
    <script>
        document.getElementById('autofill-btn').addEventListener('click', async function() {
            const urlInput = document.getElementById('channel_url');
            const url = urlInput.value.trim();
            const errorDiv = document.getElementById('error-message');
            const btn = this;

            if (!url) {
                errorDiv.textContent = 'Please enter a YouTube channel URL';
                errorDiv.style.display = 'block';
                return;
            }

            // Hide error and show loading state
            errorDiv.style.display = 'none';
            btn.disabled = true;
            btn.textContent = 'Loading...';

            try {
                const response = await fetch(`/issuers/autofill?url=${encodeURIComponent(url)}`);

                if (!response.ok) {
                    const text = await response.text();
                    throw new Error(text || 'Failed to fetch channel info');
                }

                const data = await response.json();

                // Fill in the form fields
                document.getElementById('youtube_channel_id').value = data.channel_id;
                document.getElementById('channel_name').value = data.channel_name;
                document.getElementById('channel_handle').value = data.channel_handle;

                // Clear the URL input
                urlInput.value = '';

            } catch (error) {
                errorDiv.textContent = error.message;
                errorDiv.style.display = 'block';
            } finally {
                btn.disabled = false;
                btn.textContent = 'Auto-Fill';
            }
        });

        // Allow Enter key to trigger auto-fill
        document.getElementById('channel_url').addEventListener('keypress', function(e) {
            if (e.key === 'Enter') {
                e.preventDefault();
                document.getElementById('autofill-btn').click();
            }
        });
    </script>
</body>
</html>"#
            .to_string(),
    )
}

#[derive(Deserialize)]
struct CreateIssuerForm {
    youtube_channel_id: String,
    channel_name: String,
    channel_handle: Option<String>,
    verification_video_id: String,
    default_membership_label: String,
}

/// Create a new issuer
async fn create_issuer(
    State(state): State<AppState>,
    Form(form): Form<CreateIssuerForm>,
) -> Result<Response, IssuersError> {
    // Basic validation
    if form.youtube_channel_id.trim().is_empty() {
        return Err(IssuersError::ValidationError(
            "YouTube Channel ID is required".to_string(),
        ));
    }
    if form.channel_name.trim().is_empty() {
        return Err(IssuersError::ValidationError(
            "Channel Name is required".to_string(),
        ));
    }
    if form.verification_video_id.trim().is_empty() {
        return Err(IssuersError::ValidationError(
            "Verification Video ID is required".to_string(),
        ));
    }

    let channel_handle = if let Some(h) = form.channel_handle {
        if h.trim().is_empty() {
            None
        } else {
            Some(h.trim().to_string())
        }
    } else {
        None
    };

    let issuer = CardIssuer::create(
        &state.pool,
        CreateIssuerData {
            youtube_channel_id: form.youtube_channel_id.trim().to_string(),
            channel_handle,
            channel_name: form.channel_name.trim().to_string(),
            verification_video_id: form.verification_video_id.trim().to_string(),
            default_membership_label: form.default_membership_label.trim().to_string(),
        },
    )
    .await
    .map_err(IssuersError::DatabaseError)?;

    tracing::info!(issuer_id = %issuer.id, "Created new issuer");

    Ok(axum::response::Redirect::to("/issuers").into_response())
}

/// Show edit form
async fn edit_issuer_form(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Html<String>, IssuersError> {
    let issuer = CardIssuer::find_by_id(&state.pool, id)
        .await
        .map_err(IssuersError::DatabaseError)?
        .ok_or(IssuersError::NotFound)?;

    let html = format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <title>Edit Issuer - VPass</title>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <style>
        * {{ margin: 0; padding: 0; box-sizing: border-box; }}
        body {{
            font-family: 'Helvetica Neue', Arial, sans-serif;
            background: #E8E6E0;
            color: #000;
            min-height: 100vh;
            padding: 20px;
        }}
        .container {{
            max-width: 700px;
            margin: 40px auto;
        }}
        .back {{
            display: inline-block;
            color: #666;
            text-decoration: none;
            font-size: 13px;
            margin-bottom: 40px;
            transition: color 0.2s;
        }}
        .back:hover {{ color: #1E3A5F; }}
        .back::before {{ content: '← '; }}
        h1 {{
            font-size: 48px;
            font-weight: 300;
            letter-spacing: -1px;
            color: #1E3A5F;
            margin-bottom: 8px;
        }}
        .subtitle {{
            font-size: 14px;
            text-transform: uppercase;
            letter-spacing: 2px;
            color: #666;
            margin-bottom: 40px;
        }}
        form {{
            background: #F5F3ED;
            padding: 40px;
            box-shadow: 0 2px 4px rgba(0,0,0,0.08);
        }}
        .form-group {{
            margin-bottom: 32px;
        }}
        label {{
            display: block;
            font-size: 11px;
            font-weight: 600;
            text-transform: uppercase;
            letter-spacing: 1px;
            color: #666;
            margin-bottom: 12px;
        }}
        label .required {{
            color: #FF5722;
        }}
        input, textarea {{
            width: 100%;
            padding: 16px;
            background: #fff;
            border: 1px solid #CCC;
            color: #000;
            font-size: 14px;
            font-family: 'Courier New', monospace;
            transition: border-color 0.2s;
        }}
        input:focus, textarea:focus {{
            outline: none;
            border-color: #1E3A5F;
        }}
        input::placeholder, textarea::placeholder {{
            color: #999;
        }}
        .help-text {{
            font-size: 12px;
            color: #666;
            margin-top: 8px;
        }}
        .readonly {{
            background: #E8E6E0;
            cursor: not-allowed;
        }}
        button {{
            width: 100%;
            padding: 20px;
            background: #B8915F;
            color: #000;
            border: none;
            font-size: 14px;
            font-weight: 600;
            text-transform: uppercase;
            letter-spacing: 1px;
            cursor: pointer;
            transition: all 0.2s;
            box-shadow: 0 2px 4px rgba(0,0,0,0.15);
        }}
        button:hover {{
            background: #1E3A5F;
            color: #fff;
        }}
    </style>
</head>
<body>
    <div class="container">
        <a href="/issuers" class="back">Back to Issuers</a>
        <h1>Edit Issuer</h1>
        <p class="subtitle">Update issuer information</p>

        <form action="/issuers/{}" method="POST">
            <div class="form-group">
                <label for="youtube_channel_id">YouTube Channel ID</label>
                <input type="text" id="youtube_channel_id" value="{}" class="readonly" readonly>
                <p class="help-text">Channel ID cannot be changed</p>
            </div>

            <div class="form-group">
                <label for="channel_name">Channel Name</label>
                <input type="text" name="channel_name" id="channel_name" value="{}">
            </div>

            <div class="form-group">
                <label for="channel_handle">Channel Handle</label>
                <input type="text" name="channel_handle" id="channel_handle" value="{}">
            </div>

            <div class="form-group">
                <label for="verification_video_id">Verification Video ID</label>
                <input type="text" name="verification_video_id" id="verification_video_id" value="{}">
                <p class="help-text">The video ID where members post verification comments</p>
            </div>

            <div class="form-group">
                <label for="default_membership_label">Default Membership Label</label>
                <input type="text" name="default_membership_label" id="default_membership_label" value="{}">
            </div>

            <button type="submit">Update Issuer</button>
        </form>
    </div>
</body>
</html>"#,
        issuer.id,
        issuer.youtube_channel_id,
        issuer.channel_name,
        issuer.channel_handle.as_deref().unwrap_or(""),
        issuer.verification_video_id,
        issuer.default_membership_label
    );

    Ok(Html(html))
}

#[derive(Deserialize)]
struct UpdateIssuerForm {
    channel_name: Option<String>,
    channel_handle: Option<String>,
    verification_video_id: Option<String>,
    default_membership_label: Option<String>,
}

/// Update an existing issuer
async fn update_issuer(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Form(form): Form<UpdateIssuerForm>,
) -> Result<Response, IssuersError> {
    // Verify issuer exists
    let issuer = CardIssuer::find_by_id(&state.pool, id)
        .await
        .map_err(IssuersError::DatabaseError)?
        .ok_or(IssuersError::NotFound)?;

    // Update channel info if provided
    let channel_name = form.channel_name.filter(|s| !s.trim().is_empty());
    let channel_handle = form.channel_handle.filter(|s| !s.trim().is_empty());
    let default_membership_label = form
        .default_membership_label
        .filter(|s| !s.trim().is_empty());

    CardIssuer::update_channel_info(
        &state.pool,
        id,
        channel_name,
        channel_handle,
        default_membership_label,
    )
    .await
    .map_err(IssuersError::DatabaseError)?;

    // Update verification video if provided
    if let Some(video_id) = form.verification_video_id.filter(|s| !s.trim().is_empty()) {
        CardIssuer::update_verification_video(&state.pool, id, &video_id)
            .await
            .map_err(IssuersError::DatabaseError)?;
    }

    tracing::info!(issuer_id = %issuer.id, "Updated issuer");

    Ok(axum::response::Redirect::to("/issuers").into_response())
}

/// Toggle issuer active status
async fn toggle_issuer_status(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Response, IssuersError> {
    let issuer = CardIssuer::find_by_id(&state.pool, id)
        .await
        .map_err(IssuersError::DatabaseError)?
        .ok_or(IssuersError::NotFound)?;

    let new_status = !issuer.is_active;

    CardIssuer::set_active_status(&state.pool, id, new_status)
        .await
        .map_err(IssuersError::DatabaseError)?;

    tracing::info!(
        issuer_id = %issuer.id,
        is_active = new_status,
        "Toggled issuer status"
    );

    Ok(axum::response::Redirect::to("/issuers").into_response())
}

#[derive(Deserialize)]
struct AutoFillQuery {
    url: String,
}

#[derive(Serialize)]
struct AutoFillResponse {
    channel_id: String,
    channel_name: String,
    channel_handle: String,
}

/// Auto-fill channel information from YouTube URL
async fn autofill_channel(
    State(state): State<AppState>,
    Query(query): Query<AutoFillQuery>,
) -> Result<Json<AutoFillResponse>, IssuersError> {
    // Check if we have a YouTube API key configured
    let api_key = state.config.youtube_api_key.as_ref().ok_or_else(|| {
        IssuersError::ValidationError("YouTube API key not configured".to_string())
    })?;

    tracing::info!(url = %query.url, "Auto-filling channel info");

    let channel_info = youtube_channel::fetch_channel_info(&query.url, api_key)
        .await
        .map_err(IssuersError::YouTubeApiError)?;

    Ok(Json(AutoFillResponse {
        channel_id: channel_info.channel_id,
        channel_name: channel_info.channel_name,
        channel_handle: channel_info.channel_handle.unwrap_or_default(),
    }))
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/issuers", get(list_issuers).post(create_issuer))
        .route("/issuers/new", get(new_issuer_form))
        .route("/issuers/autofill", get(autofill_channel))
        .route("/issuers/:id/edit", get(edit_issuer_form))
        .route("/issuers/:id", post(update_issuer))
        .route("/issuers/:id/toggle", post(toggle_issuer_status))
}
