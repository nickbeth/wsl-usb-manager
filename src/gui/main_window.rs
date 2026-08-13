use std::{
    cell::{Cell, RefCell},
    ops::Deref,
    rc::Rc,
};

use native_windows_gui as nwg;
use nwg::{NativeUi, PartialUi};

use super::auto_attach_tab::AutoAttachTab;
use super::connected_tab::ConnectedTab;
use super::nwg_ext::{MenuExt, MenuItemEx, WindowEx};
use super::persisted_tab::PersistedTab;
use crate::{
    auto_attacher::AutoAttacher,
    win_utils::{self, DeviceNotification},
};
use crate::{gui::RESOURCES, usbipd::list_devices};
use crate::i18n::{Localizable, Translator, keys};

pub(super) trait GuiTab {
    /// Initializes the tab. The root window handle is provided.
    fn init(&self, window: &Rc<nwg::Window>);

    /// Refreshes the data displayed in the tab.
    fn refresh(&self);

    /// Injects the active translator. Default no-op for tabs that are not yet
    /// localized (stage 2 will override for PersistedTab / AutoAttachTab).
    fn set_translator(&self, _t: Rc<Translator>) {}
}

#[derive(Default)]
pub struct MainWindow {
    device_notification: Cell<DeviceNotification>,

    // Window
    pub window: Rc<nwg::Window>,
    window_layout: nwg::FlexboxLayout,
    refresh_notice: nwg::Notice,

    // Tabs
    tabs_container: nwg::TabsContainer,
    connected_tab: nwg::Tab,
    connected_tab_content: ConnectedTab,
    persisted_tab: nwg::Tab,
    persisted_tab_content: PersistedTab,
    auto_attach_tab: nwg::Tab,
    auto_attach_tab_content: AutoAttachTab,

    // File menu
    menu_file: nwg::Menu,
    menu_file_refresh: nwg::MenuItem,
    menu_file_sep1: nwg::MenuSeparator,
    menu_file_exit: nwg::MenuItem,

    // Language menu. Fixed items for stage 1 (built-in en/zh). Stage 3 will
    // add dynamically inserted remote-language entries.
    menu_language: nwg::Menu,
    menu_language_en: nwg::MenuItem,
    menu_language_zh: nwg::MenuItem,
    menu_language_sep: nwg::MenuSeparator,
    menu_language_more: nwg::MenuItem,

    // Active translator snapshot, shared with the tabs via `Rc`.
    translator: RefCell<Rc<Translator>>,
}

impl MainWindow {
    pub fn new(auto_attacher: &Rc<RefCell<AutoAttacher>>) -> Self {
        Self {
            connected_tab_content: ConnectedTab::new(auto_attacher),
            auto_attach_tab_content: AutoAttachTab::new(auto_attacher),
            ..Default::default()
        }
    }

    fn init(&self) {
        // Load persisted language preference (silent fallback to English).
        let lang = crate::i18n::settings::load_language().unwrap_or_else(|| "en".to_owned());
        let translator = Rc::new(Translator::load(&lang).unwrap_or_default());
        *self.translator.borrow_mut() = translator.clone();
        self.connected_tab_content.set_translator(translator.clone());
        self.persisted_tab_content.set_translator(translator.clone());
        self.auto_attach_tab_content.set_translator(translator);

        self.connected_tab_content.init(&self.window);
        self.persisted_tab_content.init(&self.window);
        self.auto_attach_tab_content.init(&self.window);

        // Give the connected tab a way to notify the auto attach tab that it needs to refresh
        self.connected_tab_content
            .auto_attach_notice
            .set(Some(self.auto_attach_tab_content.refresh_notice.sender()));

        let sender = self.refresh_notice.sender();
        self.device_notification.set(
            win_utils::register_usb_device_notifications(move || {
                sender.notice();
            })
            .expect("Failed to register USB device notifications"),
        );

        // Apply the active language to every built control + mark the menu.
        self.re_localize_all();
        self.update_language_menu_checks();
    }

    fn min_max_info(data: &nwg::EventData) {
        if let nwg::EventData::OnMinMaxInfo(info) = data {
            info.set_min_size(600, 410);
        }
    }

    pub fn open(&self) {
        self.window.set_visible(true);
        if self.window.is_minimized() {
            self.window.restore();
        }
        self.window.set_foreground();
    }

    pub fn close(&self) {
        self.window.set_visible(false);
    }

    fn refresh(&self) {
        let devices = list_devices();
        self.connected_tab_content.refresh_with_devices(&devices);
        self.persisted_tab_content.refresh_with_devices(&devices);
        self.auto_attach_tab_content.refresh();
    }

    fn exit() {
        nwg::stop_thread_dispatch();
    }

    /// Switches the active language: rebuilds the translator, pushes it to the
    /// tabs, persists the choice, and re-localizes the whole UI.
    fn switch_language(&self, code: &str) {
        let Some(translator) = Translator::load(code).map(Rc::new) else {
            return; // Unknown code — ignore silently.
        };
        *self.translator.borrow_mut() = translator.clone();
        self.connected_tab_content.set_translator(translator.clone());
        self.persisted_tab_content.set_translator(translator.clone());
        self.auto_attach_tab_content.set_translator(translator);

        crate::i18n::settings::save_language(code);

        self.re_localize_all();
        self.update_language_menu_checks();
    }

    /// Re-applies the active translator to every localized control (main window
    /// + all three tabs).
    fn re_localize_all(&self) {
        let translator = self.translator.borrow().clone();
        let t = &*translator;
        self.re_localize(t);
        self.connected_tab_content.re_localize(t);
        self.persisted_tab_content.re_localize(t);
        self.auto_attach_tab_content.re_localize(t);
    }

    /// Toggles the checkmark on the Language menu to reflect the active code.
    fn update_language_menu_checks(&self) {
        let code = self.translator.borrow().code();
        self.menu_language_en.set_checked(code == "en");
        self.menu_language_zh.set_checked(code == "zh");
    }

    /// Placeholder for the "More languages..." entry. Stage 3 replaces this
    /// with the full remote-download dialog.
    fn show_more_languages_placeholder(&self) {
        let t = self.translator.borrow().clone();
        nwg::modal_info_message(
            self.window.handle,
            t.get(keys::DIALOG_TITLE_MORE_LANGUAGES),
            "Downloading additional language packages will be available in a future version.",
        );
    }
}

impl Localizable for MainWindow {
    fn re_localize(&self, t: &Translator) {
        // Window title
        self.window.set_text(t.get(keys::DIALOG_TITLE_MAIN));

        // File menu (top-level entries need MenuExt; items use MenuItemExt)
        self.menu_file.set_text(&self.window, t.get(keys::MENU_FILE_LABEL));
        self.menu_file_refresh.set_text(t.get(keys::MENU_FILE_REFRESH));
        self.menu_file_exit.set_text(t.get(keys::MENU_FILE_EXIT));

        // Language menu (item labels English / 中文 stay fixed as native names)
        self.menu_language.set_text(&self.window, t.get(keys::MENU_LANGUAGE_LABEL));
        self.menu_language_more.set_text(t.get(keys::MENU_LANGUAGE_MORE));

        // Tab titles
        self.connected_tab.set_text(t.get(keys::TAB_CONNECTED_LABEL));
        self.persisted_tab.set_text(t.get(keys::TAB_PERSISTED_LABEL));
        self.auto_attach_tab.set_text(t.get(keys::TAB_AUTO_ATTACH_LABEL));
    }
}

pub struct MainWindowUi {
    inner: Rc<MainWindow>,
    default_handler: nwg::EventHandler,
}

impl NativeUi<MainWindowUi> for MainWindow {
    fn build_ui(mut data: Self) -> Result<MainWindowUi, nwg::NwgError> {
        // Initial English text for build-time controls; the persisted language
        // is applied during `init` via `re_localize_all`.
        let t = Translator::default();

        // Controls (parent-first order)
        nwg::Window::builder()
            .flags(nwg::WindowFlags::MAIN_WINDOW | nwg::WindowFlags::VISIBLE)
            .size((780, 430))
            .center(true)
            .title(t.get(keys::DIALOG_TITLE_MAIN))
            .icon(Some(&RESOURCES.app_icon))
            .build(Rc::get_mut(&mut data.window).unwrap())?;

        nwg::Notice::builder()
            .parent(&*data.window)
            .build(&mut data.refresh_notice)?;

        nwg::TabsContainer::builder()
            .parent(&*data.window)
            .build(&mut data.tabs_container)?;

        nwg::Tab::builder()
            .parent(&data.tabs_container)
            .text(t.get(keys::TAB_CONNECTED_LABEL))
            .build(&mut data.connected_tab)?;

        nwg::Tab::builder()
            .parent(&data.tabs_container)
            .text(t.get(keys::TAB_PERSISTED_LABEL))
            .build(&mut data.persisted_tab)?;

        nwg::Tab::builder()
            .parent(&data.tabs_container)
            .text(t.get(keys::TAB_AUTO_ATTACH_LABEL))
            .build(&mut data.auto_attach_tab)?;

        nwg::Menu::builder()
            .parent(&*data.window)
            .text(t.get(keys::MENU_FILE_LABEL))
            .popup(false)
            .build(&mut data.menu_file)?;

        nwg::MenuItem::builder()
            .parent(&data.menu_file)
            .text(t.get(keys::MENU_FILE_REFRESH))
            .build(&mut data.menu_file_refresh)?;

        nwg::MenuSeparator::builder()
            .parent(&data.menu_file)
            .build(&mut data.menu_file_sep1)?;

        nwg::MenuItem::builder()
            .parent(&data.menu_file)
            .text(t.get(keys::MENU_FILE_EXIT))
            .build(&mut data.menu_file_exit)?;

        nwg::Menu::builder()
            .parent(&*data.window)
            .text(t.get(keys::MENU_LANGUAGE_LABEL))
            .popup(false)
            .build(&mut data.menu_language)?;

        // Native language names are intentionally fixed (not localized): each
        // entry always shows the language's own name regardless of the active
        // UI language, which is the conventional behaviour for language menus.
        nwg::MenuItem::builder()
            .parent(&data.menu_language)
            .text("English")
            .build(&mut data.menu_language_en)?;

        nwg::MenuItem::builder()
            .parent(&data.menu_language)
            .text("中文")
            .build(&mut data.menu_language_zh)?;

        nwg::MenuSeparator::builder()
            .parent(&data.menu_language)
            .build(&mut data.menu_language_sep)?;

        nwg::MenuItem::builder()
            .parent(&data.menu_language)
            .text(t.get(keys::MENU_LANGUAGE_MORE))
            .build(&mut data.menu_language_more)?;

        // Build partials
        ConnectedTab::build_partial(&mut data.connected_tab_content, Some(&data.connected_tab))?;
        PersistedTab::build_partial(&mut data.persisted_tab_content, Some(&data.persisted_tab))?;
        AutoAttachTab::build_partial(
            &mut data.auto_attach_tab_content,
            Some(&data.auto_attach_tab),
        )?;

        // Wrap in Rc
        let inner = Rc::new(data);
        // Bind events
        let evt_ui = Rc::downgrade(&inner);

        let window_handle = inner.window.handle;
        let default_handler =
            nwg::full_bind_event_handler(&window_handle, move |evt, evt_data, handle| {
                if let Some(ui) = evt_ui.upgrade() {
                    match evt {
                        nwg::Event::OnWindowClose => {
                            if handle == ui.window.handle {
                                if let nwg::EventData::OnWindowClose(close_data) = &evt_data {
                                    close_data.close(false);
                                }

                                MainWindow::close(&ui);
                            }
                        }
                        nwg::Event::OnInit => {
                            if handle == ui.window.handle {
                                MainWindow::init(&ui);
                            }
                        }
                        nwg::Event::OnMinMaxInfo => {
                            if handle == ui.window.handle {
                                MainWindow::min_max_info(&evt_data);
                            }
                        }
                        nwg::Event::OnNotice => {
                            if handle == ui.refresh_notice.handle {
                                MainWindow::refresh(&ui);
                            }
                        }
                        nwg::Event::OnMenuItemSelected => {
                            if handle == ui.menu_file_refresh.handle {
                                MainWindow::refresh(&ui);
                            }
                            if handle == ui.menu_file_exit.handle {
                                MainWindow::exit();
                            }
                            // Language menu
                            if handle == ui.menu_language_en.handle {
                                MainWindow::switch_language(&ui, "en");
                            }
                            if handle == ui.menu_language_zh.handle {
                                MainWindow::switch_language(&ui, "zh");
                            }
                            if handle == ui.menu_language_more.handle {
                                MainWindow::show_more_languages_placeholder(&ui);
                            }
                        }
                        _ => {}
                    }

                    // Forward events to partials
                    ui.connected_tab_content
                        .process_event(evt, &evt_data, handle);
                    ui.persisted_tab_content
                        .process_event(evt, &evt_data, handle);
                    ui.auto_attach_tab_content
                        .process_event(evt, &evt_data, handle);
                }
            });

        let ui = MainWindowUi {
            inner,
            default_handler,
        };

        // Build layouts
        nwg::FlexboxLayout::builder()
            .parent(&*ui.window)
            .auto_spacing(Some(2))
            .child(&ui.tabs_container)
            .build(&ui.window_layout)?;

        Ok(ui)
    }
}

impl Drop for MainWindowUi {
    fn drop(&mut self) {
        nwg::unbind_event_handler(&self.default_handler);
    }
}

impl Deref for MainWindowUi {
    type Target = MainWindow;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
