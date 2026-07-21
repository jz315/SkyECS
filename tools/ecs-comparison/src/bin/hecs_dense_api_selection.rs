use sky_ecs_comparison::hecs::certify_dense_apis;

fn main() {
    let certification = certify_dense_apis(4);
    println!(
        "{}",
        serde_json::to_string_pretty(&certification)
            .expect("hecs dense API certification result must serialize")
    );
}
