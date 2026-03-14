use panex::config::PanexConfig;

/// Helper: build a config with a specific panel_width
fn config_with_panel_width(panel_width: Option<u16>) -> PanexConfig {
    PanexConfig::from_args(
        vec!["echo test".to_string()],
        None,
        false,
        500,
        10_000,
        panel_width,
    )
}

#[test]
fn default_panel_width_uses_20_columns() {
    let config = config_with_panel_width(None);
    // Default: no panel_width → fixed 20 columns regardless of terminal width
    assert_eq!(config.compute_panel_columns(100), 20);
    assert_eq!(config.compute_panel_columns(200), 20);
    assert_eq!(config.compute_panel_columns(80), 20);
}

#[test]
fn panel_width_30_uses_30_percent() {
    let config = config_with_panel_width(Some(30));
    // 30% of 100 = 30 columns
    assert_eq!(config.compute_panel_columns(100), 30);
    // 30% of 200 = 60 columns
    assert_eq!(config.compute_panel_columns(200), 60);
    // 30% of 80 = 24 columns
    assert_eq!(config.compute_panel_columns(80), 24);
}

#[test]
fn panel_width_below_minimum_clamps_to_10() {
    let config = config_with_panel_width(Some(5));
    // Clamped to 10%, so 10% of 100 = 10 columns
    assert_eq!(config.compute_panel_columns(100), 10);
    // 10% of 200 = 20 columns
    assert_eq!(config.compute_panel_columns(200), 20);
}

#[test]
fn panel_width_above_maximum_clamps_to_50() {
    let config = config_with_panel_width(Some(60));
    // Clamped to 50%, so 50% of 100 = 50 columns
    assert_eq!(config.compute_panel_columns(100), 50);
    // 50% of 200 = 100 columns
    assert_eq!(config.compute_panel_columns(200), 100);
}

#[test]
fn panel_width_at_boundary_10_percent() {
    let config = config_with_panel_width(Some(10));
    assert_eq!(config.compute_panel_columns(100), 10);
}

#[test]
fn panel_width_at_boundary_50_percent() {
    let config = config_with_panel_width(Some(50));
    assert_eq!(config.compute_panel_columns(100), 50);
}

#[test]
fn gutter_and_output_offsets_with_panel_width() {
    let config = config_with_panel_width(Some(30));
    let panel_cols = config.compute_panel_columns(100); // 30
                                                        // Gutter starts at panel_cols - 1 (last column of process list)
    let gutter_start = panel_cols - 1;
    // Output panel starts at panel_cols + 1 (after delimiter)
    let output_panel_x = panel_cols + 1;
    assert_eq!(gutter_start, 29);
    assert_eq!(output_panel_x, 31);
}

#[test]
fn gutter_and_output_offsets_default() {
    let config = config_with_panel_width(None);
    let panel_cols = config.compute_panel_columns(100); // 20
    let gutter_start = panel_cols - 1;
    let output_panel_x = panel_cols + 1;
    assert_eq!(gutter_start, 19);
    assert_eq!(output_panel_x, 21);
}
