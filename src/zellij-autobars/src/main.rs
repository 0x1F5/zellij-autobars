// バーの出し入れを自動でする zellij プラグイン。load_plugins で裏で動かす。
//   locked でタブ 1 つ      → バーを全部隠す
//   locked でタブ 2 つ以上  → ステータスバー（ショートカット一覧）だけ隠す
//   locked 以外（Ctrl g 後） → 全部出す
use std::collections::BTreeMap;
use zellij_tile::prelude::*;

const STATUS_BAR: &str = "status-bar";

#[derive(Clone, Copy, PartialEq)]
enum Hidden {
    Nothing,
    StatusBar,
    All,
}

#[derive(Default)]
struct State {
    granted: bool,
    // 最後に指示した状態
    hidden: Option<Hidden>,
    tab_count: usize,
    locked: bool,
}

register_plugin!(State);

impl ZellijPlugin for State {
    fn load(&mut self, _configuration: BTreeMap<String, String>) {
        // 起動直後のモード通知は、許可が下りる前に来て受け取れないことがある。
        // default_mode "locked" で使うので、最初は locked とみなす
        self.locked = true;
        request_permission(&[
            PermissionType::ReadApplicationState,
            PermissionType::ChangeApplicationState,
        ]);
        subscribe(&[
            EventType::TabUpdate,
            EventType::ModeUpdate,
            EventType::PermissionRequestResult,
        ]);
    }

    fn update(&mut self, event: Event) -> bool {
        match event {
            Event::PermissionRequestResult(PermissionStatus::Granted) => {
                self.granted = true;
                self.apply();
            }
            Event::TabUpdate(tabs) => {
                self.tab_count = tabs.len();
                self.apply();
            }
            Event::ModeUpdate(mode_info) => {
                self.locked = mode_info.mode == InputMode::Locked;
                self.apply();
            }
            _ => {}
        }
        false
    }

    fn render(&mut self, _rows: usize, _cols: usize) {}
}

impl State {
    fn apply(&mut self) {
        if !self.granted || self.tab_count == 0 {
            return;
        }
        let want = match (self.locked, self.tab_count) {
            (false, _) => Hidden::Nothing,
            (true, 1) => Hidden::All,
            (true, _) => Hidden::StatusBar,
        };
        // 切り替わるときだけ指示する。手で出したバーを毎回隠し直さないため
        if self.hidden == Some(want) {
            return;
        }
        // HideBars は隠す対象を足していくだけなので、減らすときは一度全部出す
        let shrinking = matches!(
            (self.hidden, want),
            (Some(Hidden::All), Hidden::StatusBar) | (_, Hidden::Nothing)
        );
        if shrinking || self.hidden.is_none() {
            show_bars();
        }
        match want {
            Hidden::Nothing => {}
            Hidden::StatusBar => hide_bars(vec![STATUS_BAR.to_owned()]),
            // zellij は画面の端に接した帯しかバーとみなさない。
            // ステータスバーを先に隠すと、その上の zjstatus が下端に来て隠せるようになる
            Hidden::All => {
                hide_bars(vec![STATUS_BAR.to_owned()]);
                hide_bars(vec![]);
            }
        }
        self.hidden = Some(want);
    }
}
