use regex::Regex;

const ALLOW_LIST: [&str; 68] = [
    r"localhost",                               // local proxies
    r"audio-sp-.*\.pscdn\.co",                  // audio
    r"audio-fa\.scdn\.co",                      // audio
    r"audio4-fa\.scdn\.co",                     // audio
    r"charts-images\.scdn\.co",                 // charts images
    r"daily-mix\.scdn\.co",                     // daily mix images
    r"dailymix-images\.scdn\.co",               // daily mix images
    r"heads-fa\.scdn\.co",                      // audio (heads)
    r"i\.scdn\.co",                             // cover art
    r"lineup-images\.scdn\.co",                 // playlists lineup images
    r"merch-img\.scdn\.co",                     // merch images
    r"misc\.scdn\.co",                          // miscellaneous images
    r"mosaic\.scdn\.co",                        // playlist mosaic images
    r"newjams-images\.scdn\.co",                // release radar images
    r"o\.scdn\.co",                             // cover art
    r"pl\.scdn\.co",                            // playlist images
    r"profile-images\.scdn\.co",                // artist profile images
    r"seeded-session-images\.scdn\.co",         // radio images
    r"t\.scdn\.co",                             // background images
    r"thisis-images\.scdn\.co",                 // "this is" playlists images
    r"video-fa\.scdn\.co",                      // videos
    r"content\.production\.cdn\.art19\.com",    // podcasts
    r"rss\.art19\.com",                         // podcasts
    r".*\.buzzsprout\.com",                     // podcasts
    r"chtbl\.com",                              // podcasts
    r"platform-lookaside\.fbsbx\.com",          // Facebook profile images
    r"genius\.com",                             // lyrics (genius-spicetify)
    r".*\.googlevideo\.com",                    // YouTube videos (Spicetify Reddit app)
    r".*\.gvt1\.com",                           // Widevine download
    r"hwcdn\.libsyn\.com",                      // podcasts
    r"traffic\.libsyn\.com",                    // podcasts
    r"api.*-desktop\.musixmatch\.com",          // lyrics (genius-spicetify)
    r".*\.podbean\.com",                        // podcasts
    r"cdn\.podigee\.com",                       // podcasts
    r"dts\.podtrac\.com",                       // podcasts
    r"www\.podtrac\.com",                       // podcasts
    r"www\.reddit\.com",                        // Reddit (Spicetify Reddit app)
    r"audio\.simplecast\.com",                  // podcasts
    r"media\.simplecast\.com",                  // podcasts
    r"ap\.spotify\.com",                        // audio (access point)
    r".*\.ap\.spotify\.com",                    // resolved access points
    r"api\.spotify\.com",                       // client APIs
    r"api-partner\.spotify\.com",               // album/artist pages
    r"xpui\.app\.spotify\.com",                 // user interface
    r"apresolve\.spotify\.com",                 // access point resolving
    r"clienttoken\.spotify\.com",               // login
    r".*dealer\.spotify\.com",                  // websocket connections
    r"image-upload.*\.spotify\.com",            // image uploading
    r"login.*\.spotify\.com",                   // login
    r".*-spclient\.spotify\.com",               // client APIs
    r"spclient\.wg\.spotify\.com",              // client APIs, ads/tracking (blocked in blacklist)
    r"audio-fa\.spotifycdn\.com",               // audio
    r"seed-mix-image\.spotifycdn\.com",         // mix images
    r"download\.ted\.com",                      // podcasts
    r"www\.youtube\.com",                       // YouTube (Spicetify Reddit app)
    r"i\.ytimg\.com",                           // YouTube images (Spicetify Reddit app)
    r"chrt\.fm",                                // podcasts
    r"dcs.*\.megaphone\.fm",                    // podcasts
    r"traffic\.megaphone\.fm",                  // podcasts
    r"pdst\.fm",                                // podcasts
    r"audio-ak-spotify-com\.akamaized\.net",    // audio
    r"audio4-ak-spotify-com\.akamaized\.net",   // audio
    r"heads4-ak-spotify-com\.akamaized\.net",   // audio (heads)
    r"audio4-ak\.spotify\.com\.edgesuite\.net", // audio
    r"scontent.*\.fbcdn\.net",                  // Facebook profile images
    r"audio-sp-.*\.spotifycdn\.net",            // audio
    r"dovetail\.prxu\.org",                     // podcasts
    r"dovetail-cdn\.prxu\.org",                 // podcasts
];

const DENY_LIST: [&str; 3] = [
    r"https://spclient\.wg\.spotify\.com/ads/.*",      // ads
    r"https://spclient\.wg\.spotify\.com/ad-logic/.*", // ads
    r"https://spclient\.wg\.spotify\.com/gabo-receiver-service/.*", // tracking
];

lazy_static::lazy_static! {
  static ref ALLOW_LIST_REGEX: Vec<Regex> = ALLOW_LIST.iter().map(|s| Regex::new(s).unwrap()).collect();
  static ref DENY_LIST_REGEX: Vec<Regex> = DENY_LIST.iter().map(|s| Regex::new(s).unwrap()).collect();
}

pub fn should_block(url: &str) -> bool {
    #[allow(unused_variables)]
    let is_in_denylist = DENY_LIST_REGEX.iter().any(|p| p.is_match(url));
    let is_in_allowlist = ALLOW_LIST_REGEX.iter().any(|p| p.is_match(url));
    !is_in_allowlist
}
