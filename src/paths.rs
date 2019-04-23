/// Module for finding paths to useful files
use std::sync::Mutex;

lazy_static! {
    // We store a single shared reference to the kpathsea library so that we
    // don't have to spend the cost of initializing Kpathsea multiple times.
    // Also, since the kpathsea library isn't thread-safe, we keep a mutex to
    // ensure we're only ever performing operations in one thread. This will
    // store None before the library has been initialized, and Some(Kpaths)
    // once it has been set up.
    // Having a single shared reference is mostly useful during tests, when we
    // want many threads all performing work at the same time. The main
    // executable will probably be single threaded.
    static ref SHARED_KPATHS: Mutex<Option<kpathsea::Kpaths>> = Mutex::new(None);
}

/// Given a font name (like "cmr10"), returns a path to the font if it can be
/// found.
pub fn get_path_to_font(font_name: &str) -> Option<String> {
    let mut maybe_kpse = SHARED_KPATHS.lock().unwrap();

    if let Some(ref kpse) = *maybe_kpse {
        kpse.find_file(font_name)
    } else {
        match kpathsea::Kpaths::new() {
            Ok(kpse) => {
                let result = kpse.find_file(font_name);
                *maybe_kpse = Some(kpse);
                result
            }
            // If we can't initialize kpathsea successfully, just say we
            // couldn't find the font.
            Err(_) => None,
        }
    }
}
