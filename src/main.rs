use dioxus::{desktop::{LogicalSize, use_wry_event_handler}, prelude::*};

mod snap_layout;

fn main() {
    dioxus::LaunchBuilder::desktop()
        .with_cfg(dioxus::desktop::Config::new()
                .with_window(dioxus::desktop::WindowBuilder::new()
                        .with_min_inner_size(LogicalSize::new(450i32, 280i32))
                        .with_decorations(false)
                        .with_title("Snap Layout Example")
                )
        )
        .launch(|| rsx! {
            document::Link { rel: "stylesheet", href: asset!("/assets/main.css") }
            Titlebar {}
            Content {}
        });
}

#[component]
fn Titlebar() -> Element {
    // PUA codepoints for caption button icons
    // see: https://learn.microsoft.com/en-us/windows/apps/design/iconography/segoe-fluent-icons-font
    const CHROME_MINIMIZE: &str = "\u{e921}";
    const CHROME_MAXIMIZE: &str = "\u{e922}";
    const CHROME_RESTORE: &str = "\u{e923}";
    const CHROME_CLOSE: &str = "\u{e8bb}";

    let control_width = 46i32;
    let height= 45i32; // in logical pixels. 28 normally
    
    let mut hovering_maxbutton = use_signal_sync(|| false);
    
    use snap_layout::*;
    use_snap_layout(
        WindowCorner::TopRight,
        (control_width, 0).into(), // second from right
        (control_width, height).into(),
        false,
        move |evt| match evt {
            MaxButtonEvent::MouseEnter => hovering_maxbutton.set(true),
            MaxButtonEvent::MouseLeave => hovering_maxbutton.set(false),
            MaxButtonEvent::LeftButtonUp => dioxus::desktop::window().toggle_maximized() ,
            _ => (),
        }
        
    );
    
    // The text and symbols in native captions turns into a restore button when the window is maximized.
    // This section has no effect on the snap layouts thing. 
    let mut is_focused = use_signal(|| dioxus::desktop::window().is_focused());
    use_effect(move || {
        let _ = dioxus::document::document().eval(
            if is_focused() {
                "document.documentElement.setAttribute('window-focus', '')"
            } else {
                "document.documentElement.removeAttribute('window-focus')"
            }.into()
        );
    });

    // The maximize button on native captions turns into a restore button when the window is maximized.
    // This section has no effect on the snap layouts thing. 
    let mut is_maximized = use_signal(|| dioxus::desktop::window().is_maximized());
    use_wry_event_handler(move |wry_evt, _| {
        match wry_evt {
            dioxus::desktop::tao::event::Event::WindowEvent { event, .. } => {
                match event {
                    dioxus::desktop::WindowEvent::Focused(focus) => {
                        is_focused.set(*focus);
                    },
                    dioxus::desktop::WindowEvent::Resized(_) => {
                        let v = dioxus::desktop::window().is_maximized();
                        if is_maximized() != v {
                            is_maximized.set(dioxus::desktop::window().is_maximized());
                        }
                    },
                    _ => (),
                }
            },
            _ => (),
        }
    });


    let mut foxcat = use_signal(|| true);

    rsx! { 
        document::Link { rel: "stylesheet", href: asset!("/assets/titlebar.css") }
        div {
            id: "titlebar",
            height: "{height}px",
            onmousedown: |_| dioxus::desktop::window().drag(),
            div {
                id: "app-icon",
                img {
                    src: asset!("/assets/icon.png")
                }
            }
            div {
                id: "app-title",
                "Snap Layout Example"
            }
            div {
                id: "titlebar-controls",
                onmousedown: |evt| evt.stop_propagation(),
                div {
                    onclick: move |_| foxcat.toggle(),
                    font_size: "16px",
                    title: if foxcat() { "Fox" } else { "Cat" },
                    if foxcat() { "🦊" } else { "😺" }
                }
                div {
                    onclick: move |_| dioxus::desktop::window().set_minimized(true),
                    title: "Minimize",
                    "{CHROME_MINIMIZE}"
                }
                div {
                    class: if hovering_maxbutton() { "nc-hover" },
                    title: if is_maximized() { "Maximize" } else { "Restore" },
                    if is_maximized() {
                        "{CHROME_RESTORE}"
                    } else {
                        "{CHROME_MAXIMIZE}"
                    }
                }
                div {
                    onclick: move |_| dioxus::desktop::window().close(),
                    title: "Close",
                    "{CHROME_CLOSE}"
                }
            }
        }
        
    }
}


#[component]
fn Content() -> Element {
    rsx! {
        div {
            id: "content",
            h1 { "Snap layout flyout" }
            p { "An invisible child window is created over the maximize button, and certain events are forwarded give the appearance that it doesn't block interactions with the webview below." }
        }
    }
}
