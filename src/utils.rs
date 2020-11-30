use crate::errors::ReddSaverError;
use crate::saved::UserSaved;

use image::DynamicImage;

use log::info;
use std::fs;

#[allow(dead_code)]
pub async fn get_images(saved: &UserSaved) -> Result<(), ReddSaverError> {
    for child in saved.data.children.iter() {
        let child_cloned = child.clone();
        if let Some(url) = child_cloned.data.url {
            let extension = String::from(url.split('.').last().unwrap_or("unknown"));
            let subreddit = child_cloned.data.subreddit;
            if extension == "jpg" || extension == "png" {
                info!("Downloading image from URL: {}", url);
                let image_bytes = reqwest::get(&url).await?.bytes().await?;
                let image = match image::load_from_memory(&image_bytes) {
                    Ok(image) => image,
                    Err(_e) => return Err(ReddSaverError::CouldNotCreateImageError),
                };
                let file_name = save_image(&image, &url, &subreddit, &extension)?;
                info!("Successfully saved image: {}", file_name);
            }
        }
    }

    Ok(())
}

fn save_image(
    image: &DynamicImage,
    url: &str,
    subreddit: &str,
    extension: &str,
) -> Result<String, ReddSaverError> {
    match fs::create_dir_all(format!("data/{}", subreddit)) {
        Ok(_) => (),
        Err(_e) => return Err(ReddSaverError::CouldNotCreateDirectory),
    }

    let hash = md5::compute(url);
    let file_name = format!("data/{}/img-{:x}.{}", subreddit, hash, extension);
    match image.save(&file_name) {
        Ok(_) => (),
        Err(_e) => return Err(ReddSaverError::CouldNotSaveImageError),
    }

    Ok(file_name)
}

pub async fn get_images_parallel(saved: &UserSaved) -> Result<(), ReddSaverError> {
    let tasks: Vec<_> = saved
        .data
        .children
        .clone()
        .into_iter()
        .filter(|item| item.data.url.is_some())
        .filter(|item| {
            let url_unwrapped = item.data.url.as_ref().unwrap();
            url_unwrapped.ends_with("jpg") || url_unwrapped.ends_with("png")
        })
        .map(|item| {
            tokio::spawn(async {
                let url = item.data.url.unwrap();
                let extension = String::from(url.split('.').last().unwrap_or("unknown"));
                let subreddit = item.data.subreddit;
                info!("Downloading image from URL: {}", url);
                let image_bytes = reqwest::get(&url).await?.bytes().await?;
                let image = match image::load_from_memory(&image_bytes) {
                    Ok(image) => image,
                    Err(_e) => return Err(ReddSaverError::CouldNotCreateImageError),
                };
                let file_name = save_image(&image, &url, &subreddit, &extension)?;
                info!("Successfully saved image: {}", file_name);
                Ok::<(), ReddSaverError>(())
            })
        })
        .collect();

    for task in tasks {
        if let Err(e) = task.await? {
            return Err(e);
        }
    }

    Ok(())
}
