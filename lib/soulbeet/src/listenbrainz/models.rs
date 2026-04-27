use serde::Deserialize;

// --- Listens ---

#[derive(Debug, Deserialize, Default)]
pub struct ListensResponse {
    pub payload: ListensPayload,
}

#[derive(Debug, Deserialize, Default)]
pub struct ListensPayload {
    #[serde(default)]
    pub listens: Vec<LbListen>,
}

#[derive(Debug, Deserialize)]
pub struct LbListen {
    pub listened_at: i64,
    pub track_metadata: TrackMetadata,
}

#[derive(Debug, Deserialize)]
pub struct TrackMetadata {
    pub artist_name: String,
    pub track_name: String,
    #[serde(default)]
    pub release_name: Option<String>,
    #[serde(default)]
    pub additional_info: Option<TrackAdditionalInfo>,
}

#[derive(Debug, Deserialize)]
pub struct TrackAdditionalInfo {
    #[serde(default)]
    pub recording_mbid: Option<String>,
    #[serde(default)]
    pub artist_mbids: Option<Vec<String>>,
}

// --- Stats: Top Artists ---

#[derive(Debug, Deserialize, Default)]
pub struct TopArtistsResponse {
    pub payload: TopArtistsPayload,
}

#[derive(Debug, Deserialize, Default)]
pub struct TopArtistsPayload {
    #[serde(default)]
    pub artists: Vec<LbArtist>,
}

#[derive(Debug, Deserialize)]
pub struct LbArtist {
    pub artist_name: String,
    #[serde(default)]
    pub artist_mbid: Option<String>,
    pub listen_count: u64,
}

// --- Stats: Top Recordings ---

#[derive(Debug, Deserialize, Default)]
pub struct TopRecordingsResponse {
    pub payload: TopRecordingsPayload,
}

#[derive(Debug, Deserialize, Default)]
pub struct TopRecordingsPayload {
    #[serde(default)]
    pub recordings: Vec<LbRecording>,
}

#[derive(Debug, Deserialize)]
pub struct LbRecording {
    pub track_name: String,
    pub artist_name: String,
    #[serde(default)]
    pub recording_mbid: Option<String>,
    pub listen_count: u64,
}

// --- Similar Users ---

/// The LB API returns `{"payload": [{"user_name": "...", "similarity": 0.19}, ...]}`
#[derive(Debug, Deserialize, Default)]
pub struct SimilarUsersResponse {
    #[serde(default)]
    pub payload: Vec<SimilarUser>,
}

/// Alternative: some LB versions return a bare array. Try both shapes.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum SimilarUsersRaw {
    Wrapped(SimilarUsersResponse),
    Bare(Vec<SimilarUser>),
}

impl Default for SimilarUsersRaw {
    fn default() -> Self {
        Self::Wrapped(SimilarUsersResponse::default())
    }
}

impl SimilarUsersRaw {
    pub fn into_users(self) -> Vec<SimilarUser> {
        match self {
            Self::Wrapped(w) => w.payload,
            Self::Bare(v) => v,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct SimilarUser {
    pub user_name: String,
    pub similarity: f64,
}

// --- Recommendation Playlists (JSPF) ---

#[derive(Debug, Deserialize, Default)]
pub struct RecommendationPlaylistsResponse {
    pub playlists: Vec<JspfPlaylistWrapper>,
}

#[derive(Debug, Deserialize)]
pub struct JspfPlaylistWrapper {
    pub playlist: JspfPlaylist,
}

#[derive(Debug, Deserialize, Default)]
pub struct JspfPlaylist {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub track: Vec<JspfTrack>,
}

#[derive(Debug, Deserialize)]
pub struct JspfTrack {
    pub title: String,
    pub creator: String,
    #[serde(default)]
    pub identifier: Option<String>,
}

// --- LB Radio: Artist ---
// Response is a JSON object: { "artist_mbid": [ { recording_mbid, similar_artist_mbid, similar_artist_name, total_listen_count }, ... ] }
pub type ArtistRadioResponse = std::collections::HashMap<String, Vec<ArtistRadioRecording>>;

#[derive(Debug, Deserialize)]
pub struct ArtistRadioRecording {
    #[serde(default)]
    pub recording_mbid: Option<String>,
    #[serde(default)]
    pub similar_artist_mbid: Option<String>,
    #[serde(default)]
    pub similar_artist_name: Option<String>,
    #[serde(default)]
    pub total_listen_count: u64,
}

// --- LB Radio: Tags ---
// Response is a flat JSON array: [ { recording_mbid, percent, source, tag_count }, ... ]
pub type TagRadioResponse = Vec<TagRadioRecording>;

#[derive(Debug, Deserialize)]
pub struct TagRadioRecording {
    #[serde(default)]
    pub recording_mbid: Option<String>,
    #[serde(default)]
    pub percent: f64,
}

// --- Popularity: Artist ---

#[derive(Debug, Deserialize, Default)]
pub struct ArtistPopularityResponse(pub Vec<ArtistPopularityEntry>);

#[derive(Debug, Deserialize)]
pub struct ArtistPopularityEntry {
    #[serde(default)]
    pub artist_mbid: Option<String>,
    #[serde(default)]
    pub total_listen_count: u64,
    #[serde(default)]
    pub total_user_count: u64,
}

// --- Sitewide Artists ---

#[derive(Debug, Deserialize, Default)]
pub struct SitewideArtistsResponse {
    pub payload: SitewideArtistsPayload,
}

#[derive(Debug, Deserialize, Default)]
pub struct SitewideArtistsPayload {
    #[serde(default)]
    pub artists: Vec<SitewideArtist>,
}

#[derive(Debug, Deserialize)]
pub struct SitewideArtist {
    pub artist_name: String,
    pub listen_count: u64,
}

// --- Popularity: Top Recordings for Artist ---

#[derive(Debug, Deserialize, Default)]
pub struct TopRecordingsForArtistResponse(pub Vec<TopRecordingForArtist>);

#[derive(Debug, Deserialize)]
pub struct TopRecordingForArtist {
    pub artist_name: String,
    #[serde(default)]
    pub recording_name: Option<String>,
    #[serde(default)]
    pub recording_mbid: Option<String>,
    #[serde(default)]
    pub total_listen_count: u64,
    #[serde(default)]
    pub total_user_count: u64,
}

// --- Metadata: Artist ---

#[derive(Debug, Deserialize, Default)]
pub struct ArtistMetadataResponse(pub Vec<ArtistMetadataEntry>);

#[derive(Debug, Deserialize)]
pub struct ArtistMetadataEntry {
    #[serde(default)]
    pub artist_mbid: Option<String>,
    #[serde(default)]
    pub tag: Option<ArtistTags>,
}

#[derive(Debug, Deserialize)]
pub struct ArtistTags {
    #[serde(default)]
    pub artist: Vec<ArtistTagEntry>,
}

#[derive(Debug, Deserialize)]
pub struct ArtistTagEntry {
    pub tag: String,
    pub count: i64,
    #[serde(default)]
    pub genre_mbid: Option<String>,
}
