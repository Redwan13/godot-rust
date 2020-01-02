use crate::gamestate::GameState;
use crate::util;
use gdnative::*;

#[derive(NativeClass)]
#[inherit(Control)]
pub struct Lobby;

#[methods]
impl Lobby {
    fn _init(_owner: gdnative::Control) -> Self {
        Lobby {}
    }

    #[export]
    unsafe fn _ready(&mut self, owner: Control) {
        let emitter = &mut owner
            .get_node(NodePath::from_str("/root/gamestate"))
            .unwrap();

        util::connect_signal(emitter, &owner, "connection_failed", "on_connection_failed");
        util::connect_signal(
            emitter,
            &owner,
            "connection_succeeded",
            "on_connection_succeeded",
        );
        util::connect_signal(emitter, &owner, "player_list_changed", "refresh_lobby");
        util::connect_signal(emitter, &owner, "game_ended", "on_game_ended");
        util::connect_signal(emitter, &owner, "game_error", "on_game_error");
    }

    #[export]
    unsafe fn on_connection_failed(&self, owner: Control) {
        util::get_node::<Button>(&owner, "connect/host")
            .unwrap()
            .set_disabled(false);
        util::get_node::<Button>(&owner, "connect/join")
            .unwrap()
            .set_disabled(false);
        util::get_node::<Label>(&owner, "connect/error_label")
            .unwrap()
            .set_text(GodotString::from_str("Connection failed."));
    }

    #[export]
    unsafe fn on_connection_succeeded(&self, owner: Control) {
        util::get_node::<Control>(&owner, "connect").unwrap().hide();
        util::get_node::<Control>(&owner, "players").unwrap().show();
    }

    #[export]
    unsafe fn on_game_ended(&self, mut owner: Control) {
        owner.show();
        util::get_node::<Control>(&owner, "connect").unwrap().show();
        util::get_node::<Control>(&owner, "players").unwrap().hide();
        util::get_node::<Button>(&owner, "connect/host")
            .unwrap()
            .set_disabled(false);
    }

    #[export]
    unsafe fn on_game_error(&self, owner: Control, errmsg: GodotString) {
        let mut error = &mut util::get_node::<AcceptDialog>(&owner, "error").unwrap();
        error.set_text(errmsg);
        error.popup_centered_minsize(Vector2::new(0., 0.));
    }

    #[export]
    unsafe fn refresh_lobby(&self, owner: Control) {
        let gamestate = util::get_node::<Node>(&owner, "/root/gamestate").unwrap();

        let inst = Instance::<GameState>::try_from_base(gamestate).unwrap();

        let (mut players, player_name) = inst
            .map(|this, _owner| {
                return (this.get_player_list(), this.get_player_name());
            })
            .ok()
            .unwrap();

        players.sort();

        let mut players_list = &mut util::get_node::<ItemList>(&owner, "players/list").unwrap();
        players_list.clear();
        players_list.add_item(
            GodotString::from_str(format!("{} (You)", player_name)),
            None,
            true,
        );
        for p in players.iter() {
            players_list.add_item(GodotString::from_str(p), None, true);
        }

        let is_server = owner.get_tree().unwrap().is_network_server();
        util::get_node::<Button>(&owner, "players/start")
            .unwrap()
            .set_disabled(is_server);
    }

    #[export]
    unsafe fn _on_start_pressed(&self, owner: Control) {
        let gamestate = util::get_node::<Node>(&owner, "/root/gamestate").unwrap();
        let inst = Instance::<GameState>::try_from_base(gamestate).unwrap();
        inst.map_mut(|this, owner| this.begin_game(owner));
    }

    #[export]
    unsafe fn _on_host_pressed(&self, owner: Control) {
        let name = util::get_node::<LineEdit>(&owner, "connect/name").unwrap();
        let error = &mut util::get_node::<Label>(&owner, "connect/error_label").unwrap();
        if name.get_text().is_empty() {
            error.set_text(GodotString::from_str("Invalid name!"));
            return;
        }
        util::get_node::<Control>(&owner, "connect").unwrap().hide();
        util::get_node::<Control>(&owner, "players").unwrap().show();
        error.set_text(GodotString::new());

        let player_name = name.get_text().to_string();

        let gamestate = util::get_node::<Node>(&owner, "/root/gamestate").unwrap();
        let inst = Instance::<GameState>::try_from_base(gamestate).unwrap();
        inst.map_mut(|this, owner| {
            this.host_game(owner, player_name);
        });

        self.refresh_lobby(owner);
    }

    #[export]
    unsafe fn _on_join_pressed(&self, owner: Control) {
        let name = util::get_node::<LineEdit>(&owner, "connect/name").unwrap();
        let error = &mut util::get_node::<Label>(&owner, "connect/error_label").unwrap();
        if name.get_text().is_empty() {
            error.set_text(GodotString::from_str("Invalid name!"));
            return;
        }

        let ip = util::get_node::<LineEdit>(&owner, "connect/ip").unwrap();
        let ipval = ip.get_text();
        if !ipval.is_valid_ip_address() {
            error.set_text(GodotString::from_str("Invalid IPv4 address!"));
            return;
        }

        error.set_text(GodotString::new());
        util::get_node::<Button>(&owner, "connect/host")
            .unwrap()
            .set_disabled(true);
        util::get_node::<Button>(&owner, "connect/join")
            .unwrap()
            .set_disabled(true);

        let player_name = name.get_text().to_string();

        let gamestate = util::get_node::<Node>(&owner, "/root/gamestate").unwrap();
        let inst = Instance::<GameState>::try_from_base(gamestate).unwrap();
        inst.map_mut(|this, owner| {
            this.join_game(owner, ipval, player_name);
        });
    }
}
