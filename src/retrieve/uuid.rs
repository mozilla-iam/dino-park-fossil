use cis_client::getby::GetBy;
use cis_client::AsyncCisClientTrait;
use failure::Error;
use log::info;
use log::warn;
use lru_time_cache::LruCache;
use std::sync::Arc;
use std::sync::RwLock;

pub async fn get_uuid<T: AsyncCisClientTrait>(
    user_id: &str,
    cis_client: &T,
    cache: &Arc<RwLock<LruCache<String, String>>>,
    own: bool,
) -> Result<Option<String>, Error> {
    if !own {
        return Ok(None);
    }
    let user_id_f = user_id.to_owned();
    let cache_f = Arc::clone(cache);

    if let Some(Some(uuid)) = cache
        .write()
        .ok()
        .map(|mut c| c.get(user_id).map(Clone::clone))
    {
        Ok(Some(uuid))
    } else {
        let p = cis_client
            .get_user_by(user_id, &GetBy::UserId, None)
            .await?;
        if let Some(uuid) = p.uuid.value {
            let msg = format!("updated cache for {}", &user_id_f);
            cache_f
                .write()
                .ok()
                .map(|mut c| c.insert(user_id_f, uuid.clone()));
            info!("{}", msg);
            Ok(Some(uuid))
        } else {
            warn!("failed to look up uuid for {}", &user_id_f);
            Ok(None)
        }
    }
}
