use serde::Deserialize;

#[derive(Clone, Copy, Debug)]
pub(crate) struct TrackQuery<'a> {
    pub(crate) title: &'a str,
    pub(crate) artist: &'a str,
    pub(crate) album: Option<&'a str>,
    pub(crate) duration_seconds: Option<u64>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LyricsCandidate {
    pub(crate) id: i64,
    pub(crate) track_name: String,
    pub(crate) artist_name: String,
    pub(crate) album_name: Option<String>,
    pub(crate) duration: f64,
    pub(crate) instrumental: bool,
    pub(crate) plain_lyrics: Option<String>,
    pub(crate) synced_lyrics: Option<String>,
}
