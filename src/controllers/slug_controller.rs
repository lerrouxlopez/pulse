use rocket::response::Redirect;
use std::path::PathBuf;

// Catch-all, low-priority handler to support tournament "root" URLs:
// - GET /<slug>      -> /<slug>/dashboard
// - GET /<slug>/     -> /<slug>/dashboard
//
// Rocket route URIs can't be declared as "/<slug>/" (it rejects empty segments), so we
// use a trailing-segments capture and only redirect when there's exactly one segment.
#[get("/<path..>", rank = 200)]
pub fn slug_root(path: PathBuf) -> Option<Redirect> {
    let segments: Vec<_> = path.iter().collect();
    if segments.len() != 1 {
        return None;
    }

    let slug = segments[0].to_string_lossy().to_string();
    // Avoid redirecting reserved top-level routes if they ever fall through.
    if matches!(
        slug.as_str(),
        "" | "auth" | "dashboard" | "login" | "register" | "logout" | "static" | "t"
    ) {
        return None;
    }

    Some(Redirect::to(uri!(
        crate::controllers::dashboard_controller::tournament_dashboard(slug = slug)
    )))
}
