#![allow(clippy::wildcard_imports)]
use image::{DynamicImage, ImageFormat};
use seed::{prelude::*, *};
use std::collections::BTreeMap;
use ulid::Ulid;
use web_sys::{self, DragEvent, Event, FileList};

use rand::seq::SliceRandom;
use rand::thread_rng;

extern crate base64;
extern crate image;

const THUMB_SIZE: u32 = 250;
const COLUMNS_NUMBER: usize = 6;

const QUESTION_IMG: &str = "/matching-seed/q.png";
const ARROW_IMAGE: &str = "/matching-seed/arrow.png";

// ------ ------
//     Init
// ------ ------
fn init(_: Url, _: &mut impl Orders<Msg>) -> Model {
    Model::default()
}

// ------ ------
//     Models
// ------ ------
#[derive(PartialOrd, PartialEq)]
enum CardState {
    FaceUp,
    FaceDown,
}

enum NewCardType {
    OnePhoto(String),
    Empty,
}

#[derive(Clone)]
struct Card {
    text: Option<String>,
    photo: Option<String>,
    id: Ulid,
}

struct PlayedCard {
    card: Card,
    displayed: CardState,
    matched: bool,
}

struct Model {
    game_started: bool,
    words_list: BTreeMap<Ulid, Card>,
    board: Vec<PlayedCard>,
    last: Option<Ulid>,
    needs_reset: bool,

    // for drag and drop
    drop_zone_active: bool,
}

impl Model {
    fn all_face_down(&mut self) {
        for card in &mut self.board {
            card.displayed = CardState::FaceDown;
        }
        self.needs_reset = false;
        self.last = None;
    }
}

impl Default for Model {
    fn default() -> Self {
        Self {
            game_started: false,
            words_list: BTreeMap::new(),
            board: Vec::new(),
            last: None,
            needs_reset: false,

            drop_zone_active: false,
        }
    }
}

// ------ ------
//    Update
// ------ ------
enum Msg {
    NewCard(NewCardType),
    UpdateCardText { id: Ulid, text: String },
    DeleteCard(Ulid),
    GuessCard(usize),
    StartGame,
    ExitGame,
    ResetClick,

    DragEnter,
    DragOver,
    DragLeave,
    Drop(FileList),
}

#[cfg_attr(feature = "cargo-clippy", allow(clippy::too_many_lines))]
#[cfg_attr(
    feature = "cargo-clippy",
    allow(clippy::case_sensitive_file_extension_comparisons)
)]
// update, and make clippy allow too many lines since I don't feel like making this more readable
fn update(msg: Msg, model: &mut Model, orders: &mut impl Orders<Msg>) {
    match msg {
        // create a new card based on NewCardType
        Msg::NewCard(card_type) => {
            let new_id = Ulid::new();

            match card_type {
                NewCardType::Empty => {
                    let new_card = Card {
                        id: new_id,
                        photo: None,
                        text: None,
                    };
                    model.words_list.entry(new_id).or_insert(new_card);
                }
                NewCardType::OnePhoto(content) => {
                    let new_card = Card {
                        id: new_id,
                        photo: Some(content),
                        text: None,
                    };
                    model.words_list.entry(new_id).or_insert(new_card);
                }
            }
        }

        // update a card with new text
        Msg::UpdateCardText { id, text } => {
            if !text.is_empty() {
                if let Some(card) = model.words_list.get_mut(&id) {
                    card.text = Some(text);
                }
            }
        }

        // delete a card from the BTree
        Msg::DeleteCard(id) => {
            let _garbage = model.words_list.remove(&id);
        }

        // let me guess the card
        Msg::GuessCard(index) => {
            if model.needs_reset {
                model.all_face_down();
                return;
            }

            // do whatever based on whether there's a model.last or not
            if let Some(last_guessed) = model.last {
                // two IDs
                let just_guessed = model.board[index].card.id;
                if just_guessed == last_guessed {
                    // the person guessed correctly!
                    // set the cards to displayed and to matched = true
                    for card in &mut model.board {
                        if card.card.id == just_guessed || card.card.id == last_guessed {
                            card.displayed = CardState::FaceUp;
                            card.matched = true;
                        }
                    }
                    // set the last to none again, since it was a correct guess.
                    model.last = None;
                } else {
                    // guessed incorrectly :(
                    model.board[index].displayed = CardState::FaceUp;
                    model.needs_reset = true;
                }
            } else {
                // this will be the only flipped card, so set the last value to this one
                model.last = Some(model.board[index].card.id);
                // and flip the card so we can see it
                model.board[index].displayed = CardState::FaceUp;
            }
        }

        // start the game
        Msg::StartGame => {
            if model.words_list.len() < 2 {
                return;
            }
            let mut new_board: Vec<PlayedCard> = vec![];
            for card_pair in model.words_list.values() {
                // skip the card if both photo and text are empty
                if card_pair.text == None && card_pair.photo == None {
                    continue;
                }

                new_board.push(PlayedCard {
                    displayed: CardState::FaceDown,
                    matched: false,
                    card: card_pair.clone(),
                });
                new_board.push(PlayedCard {
                    displayed: CardState::FaceDown,
                    matched: false,
                    card: card_pair.clone(),
                });
            }

            // now shuffle it to make it random
            new_board.shuffle(&mut thread_rng());

            // copy new_board to model.board
            model.board = new_board;

            // board is made, now set the model to show the game has started
            model.game_started = true;
        }

        // set the model to all the default values to start over
        Msg::ExitGame => {
            model.words_list = BTreeMap::new();
            model.game_started = false;
            model.board = vec![];
            model.last = None;
            model.needs_reset = false;
        }

        // ResetClick will let me turn off the click listener and turn all cards FaceDown
        Msg::ResetClick => {
            // set all to face down
            model.all_face_down();
        }

        // ******
        // the following is for dragging files
        // from https://github.com/seed-rs/seed/blob/master/examples/drop_zone/src/lib.rs
        // ******
        Msg::DragEnter => model.drop_zone_active = true,

        Msg::DragOver => (),

        Msg::DragLeave => model.drop_zone_active = false,

        Msg::Drop(file_list) => {
            model.drop_zone_active = false;

            let files = (0..file_list.length())
                .filter_map(|index| {
                    let file = file_list.get(index).expect("get file with given index");
                    if file.name().to_lowercase().ends_with(".png")
                        || file.name().to_lowercase().ends_with(".gif")
                    {
                        Some(file)
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();

            for file in files {
                // go through files, process them
                orders.perform_cmd(async move {
                    let result: JsValue = wasm_bindgen_futures::JsFuture::from(file.array_buffer())
                        .await
                        .expect("expected result from promise");

                    let array: Vec<u8> = js_sys::Uint8Array::new(&result).to_vec();

                    let pic: DynamicImage =
                        image::load_from_memory(&array).expect("load pic from js array");

                    let format: ImageFormat = image::guess_format(&array).expect("guess format");

                    let pic = pic.resize(THUMB_SIZE, THUMB_SIZE, image::imageops::Gaussian);

                    // from https://stackoverflow.com/questions/57457818/how-to-convert-dynamicimage-to-base64
                    let mut blob_buf = vec![];
                    let _garbage = pic.write_to(&mut blob_buf, format);
                    let resized_pic_b64: String = base64::encode(&blob_buf);

                    // make a nice url here
                    let format_string = match format {
                        ImageFormat::Gif => "image/gif",
                        ImageFormat::Png => "image/png",
                        _ => "image",
                    };

                    let nice_url_string =
                        format!("data:{};base64,{}", format_string, resized_pic_b64);

                    Msg::NewCard(NewCardType::OnePhoto(nice_url_string))
                });
            }
        }
    }
}

// ------ ------
//     View
// ------ ------

// from https://github.com/seed-rs/seed/blob/master/examples/drop_zone/src/lib.rs
// set up drag events
trait IntoDragEvent {
    fn into_drag_event(self) -> DragEvent;
}

impl IntoDragEvent for Event {
    fn into_drag_event(self) -> DragEvent {
        self.dyn_into::<web_sys::DragEvent>()
            .expect("cannot cast given event into DragEvent")
    }
}

macro_rules! stop_and_prevent {
    { $event:expr } => {
        {
            $event.stop_propagation();
            $event.prevent_default();
        }
    };
}

fn view(model: &Model) -> Vec<Node<Msg>> {
    if model.game_started {
        game_page(model)
    } else {
        new_words_page(model)
    }
}

// play the game page
fn game_page(model: &Model) -> Vec<Node<Msg>> {
    let all_cards: Vec<Node<Msg>> = model
        .board
        .iter()
        .enumerate()
        .map(|(index, played_card)| print_card(played_card, index))
        .collect();

    // take cards and put them into divs for columns
    let mut row: Vec<Node<Msg>> = vec![];
    let mut all: Vec<Node<Msg>> = vec![];
    for (index, card) in all_cards.iter().enumerate() {
        row.push(card.clone());

        // put the correct number of cards in a row
        if (index + 1) % COLUMNS_NUMBER == 0 {
            all.push(div![C!["columns"], &row]);
            row.clear();
        }
        // for the last row if it has less than columns number
        // add empty divs as placeholders
        if index == all_cards.len() - 1 {
            let remaining = COLUMNS_NUMBER - row.len();
            for _ in 0..remaining {
                row.push(div![C!["column"]]);
            }

            all.push(div![C!["columns"], &row]);
        }
    }

    // just add a couple of buttons at the bottom to make navigation easier
    all.push(div![
        button![
            "Play again!",
            C!["button is-large is-success"],
            ev(Ev::Click, move |_| { Msg::StartGame })
        ],
        button![
            "Create New",
            C!["button is-large is-warning"],
            ev(Ev::Click, move |_| { Msg::ExitGame })
        ]
    ]);

    all
}

// print a card
fn print_card(played_card: &PlayedCard, index: usize) -> Node<Msg> {
    // make a more usable photo string
    let card_image = match &played_card.card.photo {
        Some(blob) => format!("<img src=\"{}\">", blob),
        None => format!("<img src=\"{}\">", ARROW_IMAGE),
    };
    let card_text = match &played_card.card.text {
        Some(text) => text,
        None => "",
    };
    let question_image = format!("<img src=\"{}\">", QUESTION_IMG);

    let show_card = played_card.displayed == CardState::FaceUp || played_card.matched;

    if show_card {
        div![
            C!["column"],
            div![
                C!["card"],
                div![
                    C!["card-image"],
                    figure!(C!["image is-square is-fullwidth"], raw!(&card_image),)
                ],
                div![
                    C!["card-content"],
                    div![
                        C!["media"],
                        div![C!["media-content"], p!(C!["title is-4"], card_text,)]
                    ]
                ],
                ev(Ev::Click, move |_| Msg::ResetClick),
            ]
        ]
    } else {
        div![
            C!["column"],
            div![
                C!["card"],
                div![
                    C!["card-image"],
                    figure!(C!["image is-square is-fullwidth"], raw!(&question_image),)
                ],
                div![
                    C!["card-content"],
                    div![
                        C!["media"],
                        div![C!["media-content"], p!(C!["title is-4"], index + 1,)]
                    ]
                ],
                ev(Ev::Click, move |_| Msg::GuessCard(index))
            ]
        ]
    }
}

// show the new words page
fn new_words_page(model: &Model) -> Vec<Node<Msg>> {
    /*
    the list of the words and formatted
     */
    let existing_words = model
        .words_list
        .iter()
        .map(|(id, card)| {
            /*
            information for the html: image blob and flashcard word title
             */
            let image_blob = match &card.photo {
                Some(text) => format!("<img src=\"{}\">", text),
                None => "".to_string(),
            };
            let card_text = match &card.text {
                Some(text) => text,
                None => "",
            };
            let this_id = *id;

            tr!(
                td!(div![
                    IF!(!image_blob.is_empty() => raw!(&image_blob)),
                    style![
                        St::Margin => "5px",
                    ]
                ],),
                td!(div![
                    input![
                        card_text,
                        input_ev(Ev::Input, move |word| Msg::UpdateCardText {
                            id: this_id,
                            text: word
                        }),
                    ],
                    button![
                        "delete",
                        ev(Ev::Click, move |_| Msg::DeleteCard(this_id)),
                        C!["button is-small is-danger"]
                    ],
                    style![
                        St::Margin => "5px"
                    ]
                ])
            )
        })
        .collect::<Vec<Node<Msg>>>();

    /*
    other stuff: add_new button, start_game button
     */
    let add_new_button: Node<Msg> = button![
        "Add New",
        C!["button is-large is-link"],
        ev(Ev::Click, move |_| { Msg::NewCard(NewCardType::Empty) })
    ];

    let clear_list_button: Node<Msg> = button![
        "Clear List",
        C!["button is-large is-danger"],
        ev(Ev::Click, move |_| Msg::ExitGame),
    ];

    // add a start game button
    let start_game: Node<Msg> = button![
        "Start Game",
        C!["button is-large is-success"],
        ev(Ev::Click, move |_| { Msg::StartGame })
    ];

    /*
    put it all into a Vec to return
     */
    vec![
        drag_and_drop_area(model),
        br!(),
        table![existing_words, C!["table is-striped"]],
        add_new_button,
        clear_list_button,
        br!(),
        start_game,
    ]
}

// drag and drop area
// https://github.com/seed-rs/seed/blob/master/examples/drop_zone/src/lib.rs
fn drag_and_drop_area(model: &Model) -> Node<Msg> {
    div![div![
        style![
            St::Height => px(200),
            St::Width => px(200),
            St::Margin => "auto",
            St::Background => if model.drop_zone_active { "lightgreen" } else { "lightgray" },
            St::FontFamily => "sans-serif",
            St::Display => "flex",
            St::FlexDirection => "column",
            St::JustifyContent => "center",
            St::AlignItems => "center",
            St::Border => [&px(2), "dashed", "black"].join(" ");
            St::BorderRadius => px(20),
        ],
        ev(Ev::DragEnter, |event| {
            stop_and_prevent!(event);
            Msg::DragEnter
        }),
        ev(Ev::DragOver, |event| {
            let drag_event = event.into_drag_event();
            stop_and_prevent!(drag_event);
            drag_event.data_transfer().unwrap().set_drop_effect("copy");
            Msg::DragOver
        }),
        ev(Ev::DragLeave, |event| {
            stop_and_prevent!(event);
            Msg::DragLeave
        }),
        ev(Ev::Drop, |event| {
            let drag_event = event.into_drag_event();
            stop_and_prevent!(drag_event);
            let file_list = drag_event.data_transfer().unwrap().files().unwrap();
            Msg::Drop(file_list)
        }),
        div![
            style! {
                // we don't want to fire `DragLeave` when we are dragging over drop-zone children
                St::PointerEvents => "none",
            },
            div!["Drop png or gif here"],
        ],
    ],]
}

// ------ ------
//     Start
// ------ ------
#[wasm_bindgen(start)]
pub fn start() {
    // Mount the `app` to the element with the `id` "app".
    App::start("app", init, update, view);
}
