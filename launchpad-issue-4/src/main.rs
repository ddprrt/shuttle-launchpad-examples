use std::sync::Arc;

use axum::{extract::Path, response::IntoResponse, routing::get, Extension, Json, Router};
use rand::{rngs::SmallRng, seq::SliceRandom, RngCore, SeedableRng};
use tokio::sync::Mutex;

struct Lotto<'a, R: RngCore> {
    pot: Vec<u32>,
    rng: &'a mut R,
}

impl<'a, R: RngCore> Lotto<'a, R> {
    fn new(pot_size: u32, rng: &'a mut R) -> Self {
        Self {
            pot: (1..=pot_size).collect(),
            rng,
        }
    }

    fn take(&mut self, amount: usize) -> Vec<u32> {
        self.pot.shuffle(self.rng);
        self.pot.iter().take(amount).map(|e| e.to_owned()).collect()
    }
}

type SharedState = Arc<Mutex<SmallRng>>;

async fn handler_lotto(
    Path((pot_size, amount)): Path<(u32, usize)>,
    Extension(state): Extension<SharedState>,
) -> impl IntoResponse {
    let mut rng = state.lock().await;
    let mut lotto: Lotto<'_, SmallRng> = Lotto::new(pot_size, &mut rng);
    let results = lotto.take(amount);
    Json(results)
}

#[shuttle_runtime::main]
async fn axum() -> shuttle_axum::ShuttleAxum {
    let state = Arc::new(Mutex::new(SmallRng::from_entropy()));
    let router = Router::new()
        .route("/lotto/:pot/:amount", get(handler_lotto))
        .layer(Extension(state));
    Ok(router.into())
}
