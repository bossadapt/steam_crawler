use dotenv::dotenv;
use reqwest::{self, get, StatusCode};
use rusqlite::Connection;
use scraper::{node::Element, ElementRef, Html};
use serde::{Deserialize, Serialize};
use serde_json::{self};
use std::env::var;
use std::{
    fmt::format,
    i64,
    thread::{self, sleep},
    time::Duration,
    u16, u32,
    usize::MAX,
};
//my steam id: 76561198157886937
#[derive(Debug)]
struct Visability {
    games: bool,
    freinds: bool,
    reviews: bool,
}
////////////to deal with steam json api///////////////////
#[derive(Debug, Deserialize)]
struct Data {
    response: GameRequest,
}
#[derive(Debug, Deserialize)]
struct GameRequest {
    game_count: serde_json::Number,
    #[serde(default)]
    games: Vec<Game>,
}
#[derive(Debug, Deserialize, Serialize, Clone)]
struct Game {
    appid: u32,
    #[serde(default)]
    playtime_2weeks: u16,
    #[serde(default)]
    playtime_forever: u32,
    #[serde(default)]
    is_recommended: i8,
}
/////////////////////////////////////////////////////////////
#[derive(Debug)]
struct Review {
    game_id: u32,
    is_recommended: bool,
    time_played: u32,
}
#[derive(Debug)]
struct Account {
    steam_id: u64,
    games_used: bool,
    friends_available: bool,
    friends: Vec<String>,
    games: Vec<Game>,
}
//1.25 safe scrape// can get away with 1 for quite a while
const API_CD: f32 = 1.0;
#[tokio::main]
async fn main() {
    dotenv().ok();
    let steam_api: String =
        var("STEAM_API_KEY").expect("need to have .env file with steam api in it");
    //snapsauce : 76561198048800758
    //expo : 76561198042377415
    // private profile : 76561197985588812
    println!("Final attempt to make it fully run by itself");
    let conn = Connection::open("users").unwrap();
    //          steamID| games_used | friends_available | friends_available | friends | games
    let is_new_table = conn
        .execute(
            "CREATE TABLE if not exists accounts(
            steamID    INTEGER PRIMARY KEY,
            games_used  BOOLEAN,
            friends_available  BOOLEAN,
            friends JSONB,
            games JSONB
        )",
            (),
        )
        .is_ok();
    if is_new_table {
        println!("New accounts table made");
    } else {
        println!("Accounts table already made");
    }
    let mut scrape_sleep_time: f32 = 1.00;
    let mut current_id: String = "76561198273971203".to_owned();
    while true {
        let current_friends_return = get_friend_list(&current_id, &scrape_sleep_time).await;
        scrape_sleep_time = current_friends_return.1;
        attach_friends_to_sql(&conn, &current_id, &current_friends_return.0);
        let mut found_next_id = false;
        let current_friends = strip_redundant_entries(&conn, current_friends_return.0);
        for friend in current_friends {
            let mut current_visability = get_visibility(&friend, &scrape_sleep_time).await;
            scrape_sleep_time = current_visability.1;
            if !found_next_id && current_visability.0.freinds {
                current_id = friend.clone();
                found_next_id = true;
                current_visability.0.freinds = false;
            }
            if current_visability.0.games && current_visability.0.reviews {
                println!("Adding games for {}({})", friend, scrape_sleep_time);
                scrape_sleep_time = add_scraped_data_to_sql(
                    &conn,
                    friend.clone(),
                    true,
                    current_visability.0.freinds,
                    scrape_sleep_time,
                    &steam_api,
                )
                .await;
            } else {
                println!("Avoiding games from {}", friend);
                scrape_sleep_time = add_scraped_data_to_sql(
                    &conn,
                    friend.clone(),
                    false,
                    current_visability.0.freinds,
                    scrape_sleep_time,
                    &steam_api,
                )
                .await;
            }
        }
        if !found_next_id {
            current_id = get_id_with_visable_friends(&conn);
        }
        //reset current id to one with visable freinds|| get one from the sql set
    }
}
//https://steamcommunity.com/id/Comkid/friends/

//https://steamcommunity.com/id/snapsauce/reviews
fn attach_friends_to_sql(conn: &Connection, steam_id: &String, friends: &Vec<String>) {
    let attempt = conn
        .execute(
            "UPDATE accounts SET friends = (?1) WHERE steamID = (?2)",
            (serde_json::to_string(&friends).unwrap(), steam_id),
        )
        .is_ok();
    if !attempt {
        println!("failed to add freinds")
    }
}
async fn add_scraped_data_to_sql(
    conn: &Connection,
    steam_id: String,
    scrapeable: bool,
    friends_avalible: bool,
    sleep_time: f32,
    steam_api: &String,
) -> f32 {
    let mut scraped_data: Vec<Game> = Vec::new();
    let mut sleep_time = sleep_time;
    if scrapeable {
        let reviews = get_review_list(&steam_id, &sleep_time).await;
        sleep_time = reviews.1;
        if reviews.0.len() != 0 {
            let games = get_game_list(&steam_id, &steam_api).await;
            scraped_data = combine_games_and_reviews(games, reviews.0)
        }
    }
    if scraped_data.len() == 0 {
        conn.execute(
            "INSERT INTO accounts (steamID, games_used, friends_available, friends,games) VALUES (?1, ?2, ?3,?4,?5)",
            (steam_id, false, friends_avalible,"", ""),
        ).unwrap();
        // this needs to be added with empty games
    } else {
        conn.execute(
            "INSERT INTO accounts (steamID, games_used, friends_available, friends,games) VALUES (?1, ?2, ?3,?4,?5)",
            (steam_id, true, friends_avalible,"", serde_json::to_string(&scraped_data).unwrap()),
        ).unwrap();
    }
    sleep_time
}
fn get_id_with_visable_friends(conn: &Connection) -> String {
    let query = format!("SELECT steamID FROM accounts WHERE friends_available = true LIMIT 1");
    let mut stmt = conn.prepare(&query).unwrap();
    let mut rows = stmt.query([]).unwrap();
    let mut existing_ids: Vec<u64> = Vec::new();
    while let Some(row) = rows.next().unwrap() {
        existing_ids.push(row.get(0).unwrap());
    }
    let existing_id_strings: Vec<String> =
        existing_ids.into_iter().map(|id| id.to_string()).collect();
    if existing_id_strings.clone().len() == 0 {
        panic!("ran out of people with visable freinds");
    } else {
        let friend_target = &existing_id_strings[0];
        conn.execute(
            "UPDATE accounts SET friends_available = (?1) WHERE steamID = (?2)",
            (false, friend_target),
        )
        .unwrap();
    }
    return existing_id_strings.get(0).unwrap().to_owned();
}

fn strip_redundant_entries(conn: &Connection, id_list: Vec<String>) -> Vec<String> {
    let freind_in_list = id_list.join(",");
    let query = format!(
        "SELECT steamID FROM accounts WHERE SteamID IN ({})",
        freind_in_list
    );
    let mut stmt = conn.prepare(&query).unwrap();
    let mut rows = stmt.query([]).unwrap();
    let mut existing_ids: Vec<u64> = Vec::new();
    while let Some(row) = rows.next().unwrap() {
        existing_ids.push(row.get(0).unwrap());
    }
    println!(
        "Redundant Accounts In freinds List: {}/{}",
        existing_ids.len(),
        id_list.len()
    );
    let existing_id_strings: Vec<String> =
        existing_ids.into_iter().map(|id| id.to_string()).collect();
    let mut nonexisting_ids: Vec<String> = Vec::new();
    for id in id_list {
        if !existing_id_strings.contains(&id) {
            nonexisting_ids.push(id);
        }
    }
    nonexisting_ids
}

fn combine_games_and_reviews(games: Vec<Game>, reviews: Vec<Review>) -> Vec<Game> {
    if reviews.len() == 0 {
        return games;
    }
    let mut new_games = games.clone();
    for review in reviews {
        let recommendation_i8 = match review.is_recommended {
            true => 1,
            false => -1,
        };
        let position_in_games = games
            .clone()
            .into_iter()
            .position(|game| game.appid == review.game_id)
            .unwrap_or(MAX);
        if position_in_games == MAX {
            new_games.push(Game {
                appid: review.game_id,
                playtime_2weeks: 0,
                playtime_forever: review.time_played,
                is_recommended: recommendation_i8,
            })
        } else {
            new_games[position_in_games].is_recommended = recommendation_i8;
        }
    }
    new_games
}

async fn get_raw_page(url: String, sleep_time: &f32) -> (String, f32) {
    sleep(Duration::from_secs_f32(*sleep_time));
    let mut time = *sleep_time;
    let raw_webpage: String = match reqwest::get(&url).await {
        Ok(resp) => resp.text().await.unwrap(),
        Err(er) => {
            println!("url: {} error:{}", &url, er);
            let output: String;
            if er.status() == None {
                let mut initial_error_string = er.to_string();
                let mut retry_count = 0;
                let mut resp_text: String = "".to_owned();
                while initial_error_string.contains("dns error") {
                    retry_count += 1;
                    println!(
                        "disconnected, trying again in 10 min| attempt {}",
                        retry_count
                    );
                    sleep(Duration::from_secs(600));
                    match reqwest::get(&url).await {
                        Ok(resp) => {
                            initial_error_string = "".to_owned();
                            resp_text = resp.text().await.unwrap();
                        }
                        Err(err) => {
                            initial_error_string = err.to_string();
                        }
                    }
                }
                if er.to_string().contains("Connection reset by peer") {
                    println!("disconnected, connection reset, retrying in 1 sec ");
                    sleep(Duration::from_secs(1));
                    match reqwest::get(&url).await {
                        Ok(resp) => {
                            resp_text = resp.text().await.unwrap();
                        }
                        Err(err) => {}
                    }
                }
                if er.to_string().contains("channel closed") {
                    println!("|||||||||||||upping the scraping time1|||||||||||||||||");
                    sleep(Duration::from_secs(3600));
                    time = *sleep_time + 0.03;
                    resp_text = reqwest::get(&url).await.unwrap().text().await.unwrap();
                }
                if resp_text == "".to_owned() {
                    panic!("response was empty even after retying");
                }
                output = resp_text;
            } else if er.status().unwrap() == reqwest::StatusCode::TOO_MANY_REQUESTS
                || er.status().unwrap() == reqwest::StatusCode::SERVICE_UNAVAILABLE
            {
                println!("|||||||||||||upping the scraping time2|||||||||||||||||");
                sleep(Duration::from_secs(3600));
                time = *sleep_time + 0.03;
                output = reqwest::get(&url).await.unwrap().text().await.unwrap();
            } else {
                println!("|||||||||||||retrying one time|||||||||||||||||");
                sleep(Duration::from_secs(60));
                output = reqwest::get(&url).await.unwrap().text().await.unwrap();
            }
            output
        }
    };
    return (raw_webpage, time);
}
async fn get_review_list(steam_id: &str, dyn_sleep: &f32) -> (Vec<Review>, f32) {
    let mut review_list: Vec<Review> = Vec::new();
    let url = format!("https://steamcommunity.com/profiles/{}/reviews", steam_id);
    let raw_webpage = get_raw_page(url, &dyn_sleep).await;
    let mut document = scraper::Html::parse_document(&raw_webpage.0);
    let mut dyn_sleep = raw_webpage.1;
    let page_count = get_review_page_count(document.clone());
    let mut current_page_number = 1;
    while current_page_number < page_count + 1 && page_count < 150 {
        if current_page_number > 1 {
            let url = format!(
                "https://steamcommunity.com/profiles/{}/reviews/?p={}",
                steam_id, current_page_number
            );
            let raw_webpage = get_raw_page(url, &dyn_sleep).await;
            document = scraper::Html::parse_document(&raw_webpage.0);
            dyn_sleep = raw_webpage.1;
        }
        let scraper_selector = scraper::Selector::parse("div.review_box").unwrap();
        let review_blocks = document.select(&scraper_selector);

        let mut review_a_list: Vec<(String, String)> = Vec::new();
        for element in review_blocks {
            // for recommendations + appID
            let scraper_selector1 = scraper::Selector::parse("div.title").unwrap();
            let review_a = element.select(&scraper_selector1);
            let current_a: Vec<String> = review_a.into_iter().map(|id| id.html()).collect();
            let current_a = current_a.concat();
            // for hours
            let scraper_selector2 = scraper::Selector::parse("div.hours").unwrap();
            let review_b = element.select(&scraper_selector2);
            let current_b: Vec<String> = review_b.into_iter().map(|id| id.html()).collect();
            let current_b = current_b.concat();
            review_a_list.push((current_a, current_b));
        }
        for review in review_a_list {
            // for recommendations + appID
            //<div class=\"title\"><a href=\"https://steamcommunity.com/id/ameobea/recommended/427520/\">Recommended</a></div>
            let split_one: Vec<&str> = review.0.split("recommended/").collect();
            //427520/\">Recommended</a></div>
            let split_two: Vec<&str> = split_one[1].split("/").collect();
            //427520| ">Recommended</a></div>
            let game_id = split_two[0];
            let game_id_num = game_id.parse::<u32>().unwrap();
            //">Recommended</a></div>
            let split_three: Vec<&str> = split_two[1].split(">").collect();

            let split_four: Vec<&str> = split_three[1].split("<").collect();
            let is_recommended: bool = match split_four[0] {
                "Recommended" => true,
                "Not Recommended" => false,
                _ => false,
            };
            //for hours
            let hour_one: Vec<&str> = review.1.split(">").collect();
            let mut cleaned_hour = hour_one[1].replace("/t", "");
            cleaned_hour = cleaned_hour.replace(",", "");
            let hour_two: Vec<&str> = cleaned_hour.split("hrs").collect();
            let hour_string: &str = hour_two[0].trim();
            let minutes: u32 = match hour_string {
                "" => 0,
                _ => (hour_string.parse::<f32>().unwrap_or_default() * 60 as f32) as u32,
            };
            let current_review: Review = Review {
                game_id: game_id_num,
                is_recommended,
                time_played: minutes,
            };
            review_list.push(current_review);
        }
        current_page_number += 1;
    }
    println!("      added page(s) of reviews");
    (review_list, dyn_sleep)
}
fn get_review_page_count(document: Html) -> usize {
    let mut page_count: usize = 1;
    let scraper_selector = scraper::Selector::parse("div.workshopBrowsePagingControls").unwrap();
    let page_number_blocks = document.select(&scraper_selector);
    if !&page_number_blocks.last().is_none() {
        let page_number_blocks = document.select(&scraper_selector);
        let page_number_block = page_number_blocks.last().unwrap();
        let scraper_selector = scraper::Selector::parse("a.pagelink").unwrap();
        let page_number_links: scraper::element_ref::Select<'_, '_> =
            page_number_block.select(&scraper_selector);
        let page_number_links: Vec<String> = page_number_links
            .into_iter()
            .map(|page_nubmer| page_nubmer.html())
            .collect();
        if page_number_links.len() != 0 {
            let final_page_html = page_number_links.last().unwrap();
            let first_split: Vec<&str> = final_page_html.split("p=").collect();
            let second_split: Vec<&str> = first_split[1].split("\"").collect();
            page_count = second_split[0].parse::<usize>().unwrap();
        }
    }
    //page_count += page_number_blocks_html.len();
    page_count
}
//div.gameslistitems_GamesListItemContainer_29H3o
async fn get_visibility(steam_id: &str, sleep_time: &f32) -> (Visability, f32) {
    //check that freinds, games and reviews are visable via their home profile
    let url = format!("https://steamcommunity.com/profiles/{}/", steam_id);
    let raw_webpage = get_raw_page(url, &sleep_time).await;
    let sleep_time = raw_webpage.1;
    let document = scraper::Html::parse_document(&raw_webpage.0);
    let scraper_selector = scraper::Selector::parse("div.profile_item_links").unwrap();
    let page_items = document.select(&scraper_selector);
    let mut items: Vec<String> = Vec::new();
    // items: ["games", "inventory", "screenshots", "recommended"] by the end of this loop
    // this is now updated to search for numbers
    for page_item in page_items {
        let selector = scraper::Selector::parse("div.profile_count_link").unwrap();
        //redo time
        let items_select = page_item.select(&selector);
        for item in items_select {
            let selector1 = scraper::Selector::parse("span.count_link_label").unwrap();
            let title_list_select: Vec<String> =
                item.select(&selector1).map(|count| count.html()).collect();
            let cleaned_title_list: String = title_list_select
                .concat()
                .replace("\n", "")
                .replace("\t", "");
            let spl1: Vec<&str> = cleaned_title_list.split(">").collect();
            let spl2: Vec<&str> = spl1[1].split("<").collect();
            let title = spl2[0];
            if title == "Games" || title == "Reviews" {
                let selector2 = scraper::Selector::parse("span.profile_count_link_total").unwrap();
                let count_list_select: Vec<String> =
                    item.select(&selector2).map(|count| count.html()).collect();
                let cleaned_count_list: String = count_list_select
                    .concat()
                    .replace("\n", "")
                    .replace("\t", "");
                let spl1: Vec<&str> = cleaned_count_list.split(">").collect();
                let spl2: Vec<&str> = spl1[1].split("<").collect();
                //gotta remove the comma from the thousands(was skipping profiles with over 1000)
                let count_string = spl2[0].replace(",", "");
                let count = count_string.parse::<u32>().unwrap_or_default();
                if title == "Reviews" {
                    if count > 1 && count < 1500 {
                        items.push("recommended".to_owned());
                    }
                } else {
                    if count > 0 {
                        items.push("games".to_owned());
                    }
                }
            }
        }
    }
    //check if the freind div is there
    let scraper_selector = scraper::Selector::parse("div.profile_friend_links").unwrap();
    let freinds_div = document.select(&scraper_selector);
    let freinds_div_html: Vec<String> = freinds_div
        .into_iter()
        .map(|freinds_div| freinds_div.html())
        .collect();
    let mut friends_visable = false;
    if freinds_div_html.len() != 0 {
        friends_visable = true;
    }
    let games_visable: bool = items.contains(&"games".to_owned());
    let reviews_visable: bool = items.contains(&"recommended".to_owned());
    //adding extra rules to stop wasting time trying to grab empty reviews/ only 1 review(cant see one review)
    (
        Visability {
            games: games_visable,
            reviews: reviews_visable,
            freinds: friends_visable,
        },
        sleep_time,
    )
}

async fn get_game_list(steam_id: &str, steam_api: &String) -> Vec<Game> {
    let url = format!(
        "http://api.steampowered.com/IPlayerService/GetOwnedGames/v0001/?key={}&steamid={}&format=json",
        steam_api,steam_id
    );
    let mut games: Vec<Game> = Vec::new();
    match reqwest::get(&url).await {
        Ok(resp) => {
            let resp_text = resp.text().await.unwrap();
            let data: Data = match serde_json::from_str(&resp_text) {
                Ok(data) => data,
                Err(er) => {
                    let output: Data;
                    if resp_text.contains("502 Bad Gateway")
                        || resp_text.contains("504 Gateway Time-out")
                    {
                        println!("|||| BAD API GATEWAY, TRYING AGAIN IN A MIN");
                        sleep(Duration::from_secs(60));
                        let pre_data = reqwest::get(&url).await.unwrap().text().await.unwrap();
                        output = serde_json::from_str(&pre_data).unwrap();
                    } else {
                        println!("{}", resp_text);
                        panic!("json switch failed");
                    }
                    output
                }
            };
            games = data.response.games
        }
        Err(initial_error) => {
            let mut initial_error_string = initial_error.to_string();
            //attempting to fix random gateway timeout with just a wait to do it again without much protection
            if initial_error.status() == None {
                let mut retry_count = 0;
                while initial_error_string.contains("dns error") {
                    retry_count += 1;
                    println!(
                        "disconnected, trying again in 10 min| attempt {}",
                        retry_count
                    );
                    sleep(Duration::from_secs(600));
                    match reqwest::get(&url).await {
                        Ok(resp) => {
                            initial_error_string = "".to_owned();
                            let resp_text = resp.text().await.unwrap();
                            let data: Data = serde_json::from_str(&resp_text).unwrap();
                            games = data.response.games
                        }
                        Err(err) => {
                            initial_error_string = err.to_string();
                        }
                    }
                }
            } else if initial_error.status().unwrap() == StatusCode::GATEWAY_TIMEOUT {
                println!("Trying to hit on a gateway timeout again");
                sleep(Duration::from_secs(60));
                let resp = reqwest::get(&url).await.unwrap();
                let resp_text = resp.text().await.unwrap();
                let data: Data = serde_json::from_str(&resp_text).unwrap();
                games = data.response.games
            } else {
                println!("Reqwest Error: {}", initial_error);
                panic!();
            }
        }
    }
    println!("      added owned game(s)");
    //sleep(Duration::from_secs_f32(API_CD));
    //no sleep on api
    games
}

async fn get_friend_list(steam_id: &str, sleep_time: &f32) -> (Vec<String>, f32) {
    let url = format!("https://steamcommunity.com/profiles/{}/friends/", steam_id);
    let raw_webpage = get_raw_page(url, &sleep_time).await;
    let sleep_time = raw_webpage.1;
    let document = scraper::Html::parse_document(&raw_webpage.0);
    let scraper_selector = scraper::Selector::parse("div.selectable").unwrap();
    let friend_blocks = document.select(&scraper_selector);

    //turns html block to:["<a href=\"https://steamcommunity.com/id/Checker92\" data-container=\"#fr_85584701\" class=\"selectable_overlay\"></a>"]
    let mut friend_id_list: Vec<String> = Vec::new();
    for friend_block in friend_blocks {
        let freind_html = friend_block.html();
        let split_one: Vec<&str> = freind_html.split("data-steamid=\"").collect();
        let split_two: Vec<&str> = split_one[1].split("\"").collect();
        let freind_id = split_two[0];
        friend_id_list.push(freind_id.to_owned());
    }
    (friend_id_list, sleep_time)
}
