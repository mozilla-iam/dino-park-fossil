use cis_client::getby::GetBy;
use cis_client::AsyncCisClientTrait;
use failure::Error;
use futures::future::Either;
use futures::future::IntoFuture;
use futures::Future;
use log::info;
use log::warn;
use lru_time_cache::LruCache;
use std::sync::Arc;
use std::sync::RwLock;

pub fn get_uuid<T: AsyncCisClientTrait>(
    user_id: &str,
    cis_client: &T,
    cache: &Arc<RwLock<LruCache<String, String>>>,
    own: bool,
) -> impl Future<Item = Option<String>, Error = Error> {
    if !own {
        return Either::A(Ok(None).into_future());
    }
    let user_id_f = user_id.to_owned();
    let cache_f = Arc::clone(cache);

    if let Some(Some(uuid)) = cache
        .write()
        .ok()
        .map(|mut c| c.get(user_id).map(Clone::clone))
    {
        Either::A(Ok(Some(uuid)).into_future())
    } else {
        Either::B(
            cis_client
                .get_user_by(user_id, &GetBy::UserId, None)
                .map_err(Into::into)
                .map(|p| p.uuid.value)
                .map(move |uuid| {
                    if let Some(uuid) = uuid {
                        let msg = format!("updated cache for {}", &user_id_f);
                        cache_f
                            .write()
                            .ok()
                            .map(|mut c| c.insert(user_id_f, uuid.clone()));
                        info!("{}", msg);
                        Some(uuid)
                    } else {
                        warn!("faild to look up uuid for {}", &user_id_f);
                        None
                    }
                }),
        )
    }
}
