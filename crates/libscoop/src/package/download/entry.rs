/// One download of a package, as the manifest declares it.
///
/// A manifest URL may carry a `#/<name>` fragment that renames the download,
/// for example `https://example.com/dl?id=7#/tool.exe`. The fragment is not sent
/// to the server: it names the file that the download becomes on disk.
pub(crate) struct Download<'a> {
    /// The manifest URL as written, rename fragment included.
    pub(crate) manifest_url: &'a str,
    /// The URL to fetch, without the rename fragment.
    pub(crate) url: &'a str,
    /// The name the download takes once installed.
    ///
    /// The rename fragment when the URL has one, otherwise the last path
    /// segment of the URL.
    pub(crate) filename: String,
}

impl<'a> Download<'a> {
    pub(crate) fn new(manifest_url: &'a str) -> Download<'a> {
        let (url, rename) = match manifest_url.split_once('#') {
            Some((url, rename)) => (url, Some(rename)),
            None => (manifest_url, None),
        };

        let filename = rename
            .filter(|rename| !rename.is_empty())
            .and_then(|rename| {
                rename
                    .trim_start_matches(['/', '\\'])
                    .rsplit(['/', '\\'])
                    .next()
            })
            .filter(|rename| !rename.is_empty())
            .map(str::to_owned)
            .unwrap_or_else(|| url_leaf(url));

        Download {
            manifest_url,
            url,
            filename,
        }
    }
}

/// The last path segment of `url`, or `download` when it has none.
fn url_leaf(url: &str) -> String {
    let url = url.split_once('#').map(|(head, _)| head).unwrap_or(url);
    let url = url.split_once('?').map(|(head, _)| head).unwrap_or(url);

    url.trim_end_matches('/')
        .rsplit(['/', '\\'])
        .next()
        .filter(|segment| !segment.is_empty())
        .unwrap_or("download")
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::url_leaf;

    #[test]
    fn url_leaf_is_the_last_path_segment() {
        assert_eq!(
            url_leaf("https://example.com/releases/tool.zip"),
            "tool.zip"
        );
        assert_eq!(url_leaf("https://example.com/releases/"), "releases");
        assert_eq!(url_leaf("https://example.com"), "example.com");
        assert_eq!(url_leaf("https://example.com/dl?id=7"), "dl");
        assert_eq!(url_leaf("https://example.com/tool.zip?raw=1"), "tool.zip");
    }

    #[test]
    fn url_leaf_never_keeps_a_fragment() {
        assert_eq!(url_leaf("https://example.com/get#/dl.7z"), "get");
    }
}
