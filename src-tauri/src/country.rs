use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Cursor,
    path::{Path, PathBuf},
};

use base64::{Engine as _, engine::general_purpose::STANDARD};
use png::{BitDepth, ColorType, Encoder};
use serde::Serialize;

use crate::encoding::decode_windows_1252;

const LOCALISATION_DIR: &str = "localisation";
const FLAGS_DIR: [&str; 2] = ["gfx", "flags"];
const PNG_DATA_URL_PREFIX: &str = "data:image/png;base64,";

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CountryCatalogView {
    pub countries: BTreeMap<String, CountryView>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CountryView {
    pub tag: String,
    pub name: String,
    pub flag_data_url: Option<String>,
}

#[derive(Debug, Clone)]
struct DataSource {
    label: &'static str,
    localisation_dir: Option<PathBuf>,
    flags_dir: Option<PathBuf>,
}

pub fn resolve_country_catalog(
    game_path: String,
    mod_path: Option<String>,
    country_tags: Vec<String>,
) -> Result<CountryCatalogView, String> {
    let requested_tags: BTreeSet<_> = country_tags
        .into_iter()
        .map(|tag| tag.trim().to_string())
        .filter(|tag| !tag.is_empty())
        .collect();

    let mut warnings = Vec::new();
    let sources = build_sources(
        Path::new(&game_path),
        mod_path
            .as_deref()
            .map(str::trim)
            .filter(|path| !path.is_empty())
            .map(Path::new),
        &mut warnings,
    );

    let mut per_source_names = Vec::with_capacity(sources.len());
    for source in &sources {
        per_source_names.push(load_country_names(source, &requested_tags, &mut warnings));
    }

    let mut countries = BTreeMap::new();
    for tag in requested_tags {
        let name = per_source_names
            .iter()
            .find_map(|names| names.get(&tag))
            .cloned()
            .unwrap_or_else(|| tag.clone());
        let flag_data_url = resolve_flag_data_url(&tag, &sources, &mut warnings);

        countries.insert(
            tag.clone(),
            CountryView {
                tag,
                name,
                flag_data_url,
            },
        );
    }

    Ok(CountryCatalogView {
        countries,
        warnings,
    })
}

fn build_sources(
    game_path: &Path,
    mod_path: Option<&Path>,
    warnings: &mut Vec<String>,
) -> Vec<DataSource> {
    let mut sources = Vec::with_capacity(2);

    if let Some(mod_path) = mod_path {
        sources.push(describe_source("mod folder", mod_path, warnings));
    }

    sources.push(describe_source("base game folder", game_path, warnings));
    sources
}

fn describe_source(label: &'static str, root: &Path, warnings: &mut Vec<String>) -> DataSource {
    if !root.exists() {
        warnings.push(format!("{label} does not exist: {}", root.display()));
    } else if !root.is_dir() {
        warnings.push(format!("{label} is not a directory: {}", root.display()));
    }

    let localisation_dir = validate_subdirectory(label, root, LOCALISATION_DIR, warnings);
    let flags_path = root.join(FLAGS_DIR[0]).join(FLAGS_DIR[1]);
    let flags_dir = if flags_path.is_dir() {
        Some(flags_path)
    } else {
        warnings.push(format!(
            "{label} is missing a flags folder at {}",
            flags_path.display()
        ));
        None
    };

    DataSource {
        label,
        localisation_dir,
        flags_dir,
    }
}

fn validate_subdirectory(
    label: &'static str,
    root: &Path,
    child: &str,
    warnings: &mut Vec<String>,
) -> Option<PathBuf> {
    let path = root.join(child);
    if path.is_dir() {
        Some(path)
    } else {
        warnings.push(format!("{label} is missing {child} at {}", path.display()));
        None
    }
}

fn load_country_names(
    source: &DataSource,
    requested_tags: &BTreeSet<String>,
    warnings: &mut Vec<String>,
) -> BTreeMap<String, String> {
    let Some(localisation_dir) = source.localisation_dir.as_ref() else {
        return BTreeMap::new();
    };

    let mut csv_paths = match fs::read_dir(localisation_dir) {
        Ok(entries) => entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                path.extension()
                    .and_then(|extension| extension.to_str())
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("csv"))
            })
            .collect::<Vec<_>>(),
        Err(error) => {
            warnings.push(format!(
                "Failed to read {} localisation files from {}: {error}",
                source.label,
                localisation_dir.display()
            ));
            return BTreeMap::new();
        }
    };

    csv_paths.sort();

    let mut names = BTreeMap::new();
    for csv_path in csv_paths {
        if names.len() == requested_tags.len() {
            break;
        }

        let file_bytes = match fs::read(&csv_path) {
            Ok(bytes) => bytes,
            Err(error) => {
                warnings.push(format!(
                    "Failed to read localisation file {}: {error}",
                    csv_path.display()
                ));
                continue;
            }
        };

        for line in decode_windows_1252(&file_bytes).lines() {
            let Some((tag, english_name)) = parse_localisation_line(line) else {
                continue;
            };

            if !requested_tags.contains(tag) || names.contains_key(tag) {
                continue;
            }

            names.insert(tag.to_string(), english_name.to_string());
        }
    }

    names
}

fn parse_localisation_line(line: &str) -> Option<(&str, &str)> {
    let trimmed_line = line.trim();
    if trimmed_line.is_empty() || trimmed_line.starts_with('#') {
        return None;
    }

    let mut fields = trimmed_line.split(';');
    let tag = fields.next()?.trim().trim_start_matches('\u{feff}');
    let english_name = fields.next()?.trim();

    if tag.is_empty() || english_name.is_empty() {
        return None;
    }

    Some((tag, english_name))
}

fn resolve_flag_data_url(
    tag: &str,
    sources: &[DataSource],
    warnings: &mut Vec<String>,
) -> Option<String> {
    for source in sources {
        let Some(flags_dir) = source.flags_dir.as_ref() else {
            continue;
        };

        let flag_path = flags_dir.join(format!("{tag}.tga"));
        if !flag_path.is_file() {
            continue;
        }

        match fs::read(&flag_path)
            .map_err(|error| error.to_string())
            .and_then(|bytes| tga_to_png_data_url(&bytes))
        {
            Ok(data_url) => return Some(data_url),
            Err(error) => warnings.push(format!(
                "Failed to decode flag {} from {}: {error}",
                tag,
                flag_path.display()
            )),
        }
    }

    None
}

fn tga_to_png_data_url(bytes: &[u8]) -> Result<String, String> {
    let image = decode_tga(bytes)?;
    let png = encode_png(&image)?;
    Ok(format!("{PNG_DATA_URL_PREFIX}{}", STANDARD.encode(png)))
}

fn decode_tga(bytes: &[u8]) -> Result<DecodedImage, String> {
    if bytes.len() < 18 {
        return Err("TGA file is too small".to_string());
    }

    let id_length = bytes[0] as usize;
    let color_map_type = bytes[1];
    let image_type = bytes[2];
    let width = u16::from_le_bytes([bytes[12], bytes[13]]) as usize;
    let height = u16::from_le_bytes([bytes[14], bytes[15]]) as usize;
    let pixel_depth = bytes[16];
    let image_descriptor = bytes[17];

    if color_map_type != 0 {
        return Err("color-mapped TGA files are not supported".to_string());
    }

    if image_type != 2 && image_type != 10 {
        return Err(format!("unsupported TGA image type {image_type}"));
    }

    if pixel_depth != 24 && pixel_depth != 32 {
        return Err(format!("unsupported TGA pixel depth {pixel_depth}"));
    }

    if width == 0 || height == 0 {
        return Err("TGA image has zero width or height".to_string());
    }

    let bytes_per_pixel = usize::from(pixel_depth / 8);
    let pixel_count = width * height;
    let mut pixels = Vec::with_capacity(pixel_count * 4);
    let mut cursor = 18 + id_length;

    if cursor > bytes.len() {
        return Err("TGA header points past the end of the file".to_string());
    }

    if image_type == 2 {
        while pixels.len() < pixel_count * 4 {
            let next = bytes
                .get(cursor..cursor + bytes_per_pixel)
                .ok_or_else(|| "unexpected end of TGA pixel data".to_string())?;
            pixels.extend_from_slice(&pixel_to_rgba(next));
            cursor += bytes_per_pixel;
        }
    } else {
        while pixels.len() < pixel_count * 4 {
            let packet_header = *bytes
                .get(cursor)
                .ok_or_else(|| "unexpected end of TGA RLE packet header".to_string())?;
            cursor += 1;

            let run_length = usize::from((packet_header & 0x7F) + 1);
            if packet_header & 0x80 != 0 {
                let pixel = bytes
                    .get(cursor..cursor + bytes_per_pixel)
                    .ok_or_else(|| "unexpected end of TGA RLE pixel data".to_string())?;
                let rgba = pixel_to_rgba(pixel);
                cursor += bytes_per_pixel;
                for _ in 0..run_length {
                    pixels.extend_from_slice(&rgba);
                }
            } else {
                let byte_count = run_length * bytes_per_pixel;
                let raw_pixels = bytes
                    .get(cursor..cursor + byte_count)
                    .ok_or_else(|| "unexpected end of TGA raw packet data".to_string())?;
                cursor += byte_count;

                for pixel in raw_pixels.chunks_exact(bytes_per_pixel) {
                    pixels.extend_from_slice(&pixel_to_rgba(pixel));
                }
            }
        }
    }

    if pixels.len() != pixel_count * 4 {
        return Err("decoded TGA pixel count did not match image dimensions".to_string());
    }

    Ok(reorient_tga_pixels(
        width,
        height,
        image_descriptor,
        &pixels,
    ))
}

fn pixel_to_rgba(pixel: &[u8]) -> [u8; 4] {
    match pixel {
        [blue, green, red] => [*red, *green, *blue, 255],
        [blue, green, red, alpha] => [*red, *green, *blue, *alpha],
        _ => [0, 0, 0, 0],
    }
}

fn reorient_tga_pixels(
    width: usize,
    height: usize,
    image_descriptor: u8,
    pixels: &[u8],
) -> DecodedImage {
    let top_to_bottom = image_descriptor & 0x20 != 0;
    let right_to_left = image_descriptor & 0x10 != 0;
    let mut rgba = vec![0; pixels.len()];

    for source_index in 0..(width * height) {
        let source_row = source_index / width;
        let source_column = source_index % width;
        let target_x = if right_to_left {
            width - 1 - source_column
        } else {
            source_column
        };
        let target_y = if top_to_bottom {
            source_row
        } else {
            height - 1 - source_row
        };

        let source_offset = source_index * 4;
        let target_offset = (target_y * width + target_x) * 4;
        rgba[target_offset..target_offset + 4]
            .copy_from_slice(&pixels[source_offset..source_offset + 4]);
    }

    DecodedImage {
        width: width as u32,
        height: height as u32,
        rgba,
    }
}

fn encode_png(image: &DecodedImage) -> Result<Vec<u8>, String> {
    let mut png_bytes = Vec::new();
    let mut cursor = Cursor::new(&mut png_bytes);
    let mut encoder = Encoder::new(&mut cursor, image.width, image.height);
    encoder.set_color(ColorType::Rgba);
    encoder.set_depth(BitDepth::Eight);

    let mut writer = encoder
        .write_header()
        .map_err(|error| format!("failed to write PNG header: {error}"))?;
    writer
        .write_image_data(&image.rgba)
        .map_err(|error| format!("failed to write PNG data: {error}"))?;
    drop(writer);
    drop(cursor);

    Ok(png_bytes)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DecodedImage {
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}

#[cfg(test)]
mod tests {
    use super::{
        PNG_DATA_URL_PREFIX, decode_tga, parse_localisation_line, resolve_country_catalog,
        tga_to_png_data_url,
    };
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    use png::Decoder;
    use std::{
        fs,
        io::Cursor,
        path::{Path, PathBuf},
    };

    #[test]
    fn parses_localisation_lines_using_the_first_two_fields() {
        assert_eq!(
            parse_localisation_line("\u{feff}ENG;The United Kingdom;Royaume-Uni;x"),
            Some(("ENG", "The United Kingdom"))
        );
        assert_eq!(parse_localisation_line(""), None);
        assert_eq!(parse_localisation_line("# comment"), None);
    }

    #[test]
    fn uses_the_first_matching_localisation_entry_per_source_in_alphabetical_order() {
        let test_dir = TestDir::new();
        write_file(
            &test_dir.join("base").join("localisation").join("a.csv"),
            b"ENG;First Name;x\r\n",
        );
        write_file(
            &test_dir.join("base").join("localisation").join("z.csv"),
            b"ENG;Second Name;x\r\n",
        );

        let catalog = resolve_country_catalog(
            test_dir.join("base").display().to_string(),
            None,
            vec!["ENG".to_string()],
        )
        .expect("expected catalog");

        assert_eq!(catalog.countries["ENG"].name, "First Name");
    }

    #[test]
    fn prefers_mod_country_names_over_base_game_names() {
        let test_dir = TestDir::new();
        write_file(
            &test_dir
                .join("base")
                .join("localisation")
                .join("countries.csv"),
            b"ENG;United Kingdom;x\r\n",
        );
        write_file(
            &test_dir
                .join("mod")
                .join("localisation")
                .join("countries.csv"),
            b"ENG;Albion;x\r\n",
        );

        let catalog = resolve_country_catalog(
            test_dir.join("base").display().to_string(),
            Some(test_dir.join("mod").display().to_string()),
            vec!["ENG".to_string()],
        )
        .expect("expected catalog");

        assert_eq!(catalog.countries["ENG"].name, "Albion");
    }

    #[test]
    fn falls_back_to_base_game_names_and_flags_when_mod_is_missing_them() {
        let test_dir = TestDir::new();
        write_file(
            &test_dir
                .join("base")
                .join("localisation")
                .join("countries.csv"),
            b"ENG;United Kingdom;x\r\n",
        );
        write_file(
            &test_dir
                .join("base")
                .join("gfx")
                .join("flags")
                .join("ENG.tga"),
            &sample_tga_bytes(),
        );
        fs::create_dir_all(test_dir.join("mod").join("localisation"))
            .expect("expected localisation dir");
        fs::create_dir_all(test_dir.join("mod").join("gfx").join("flags"))
            .expect("expected flags dir");

        let catalog = resolve_country_catalog(
            test_dir.join("base").display().to_string(),
            Some(test_dir.join("mod").display().to_string()),
            vec!["ENG".to_string()],
        )
        .expect("expected catalog");

        let country = &catalog.countries["ENG"];
        assert_eq!(country.name, "United Kingdom");
        assert!(country.flag_data_url.is_some());
    }

    #[test]
    fn reports_missing_folders_as_warnings_without_failing_the_catalog() {
        let test_dir = TestDir::new();

        let catalog = resolve_country_catalog(
            test_dir.join("missing-base").display().to_string(),
            Some(test_dir.join("missing-mod").display().to_string()),
            vec!["ENG".to_string()],
        )
        .expect("expected catalog");

        assert_eq!(catalog.countries["ENG"].name, "ENG");
        assert!(!catalog.warnings.is_empty());
    }

    #[test]
    fn converts_tga_flags_to_png_data_urls() {
        let data_url = tga_to_png_data_url(&sample_tga_bytes()).expect("expected data url");
        assert!(data_url.starts_with(PNG_DATA_URL_PREFIX));

        let png_bytes = STANDARD
            .decode(data_url.trim_start_matches(PNG_DATA_URL_PREFIX))
            .expect("expected base64 png");
        let decoder = Decoder::new(Cursor::new(png_bytes));
        let mut reader = decoder.read_info().expect("expected PNG reader");
        let mut buffer = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut buffer).expect("expected PNG frame");

        assert_eq!(info.width, 2);
        assert_eq!(info.height, 1);
        assert_eq!(&buffer[..4], &[255, 0, 0, 255]);
        assert_eq!(&buffer[4..8], &[0, 255, 0, 255]);
    }

    #[test]
    fn decodes_rle_tga_images() {
        let image = decode_tga(&sample_tga_bytes()).expect("expected decoded image");

        assert_eq!(image.width, 2);
        assert_eq!(image.height, 1);
        assert_eq!(image.rgba, vec![255, 0, 0, 255, 0, 255, 0, 255]);
    }

    fn sample_tga_bytes() -> Vec<u8> {
        vec![
            0, 0, 10, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 0, 1, 0, 32, 8, 0x01, 0, 0, 255, 255, 0, 255,
            0, 255,
        ]
    }

    fn write_file(path: &Path, bytes: &[u8]) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("expected parent directory");
        }

        fs::write(path, bytes).expect("expected file write");
    }

    struct TestDir {
        path: PathBuf,
    }

    impl TestDir {
        fn new() -> Self {
            let unique = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("expected system time")
                .as_nanos();
            let path = std::env::temp_dir().join(format!("victoria2-war-analyzer-{unique}"));
            fs::create_dir_all(&path).expect("expected temp directory");
            Self { path }
        }

        fn join(&self, path: &str) -> PathBuf {
            self.path.join(path)
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}
