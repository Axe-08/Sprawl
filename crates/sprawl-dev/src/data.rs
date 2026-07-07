pub fn demo_triage_items() -> Vec<String> {
    vec![
        "AWS Access Key ID found in .env (high confidence)".to_string(),
        "Stripe Secret Key found in config.js (medium confidence)".to_string(),
    ]
}

pub fn demo_dashboard_state() -> String {
    "Dashboard: 2 active alerts, 100 files scanned, 40ms avg latency".to_string()
}
