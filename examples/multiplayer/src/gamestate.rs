use crate::util;
use gdnative::*;
use std::collections::HashMap;

// Default game port
const DEFAULT_PORT: i64 = 10567;

// Max number of players
const MAX_PEERS: i64 = 12;

#[derive(NativeClass)]
#[inherit(Node)]
#[register_with(Self::register_signals)]
pub struct GameState {
    player_name: String,
    players: HashMap<i64, String>,
    players_ready: Vec<i64>,
}

#[methods]
impl GameState {
    fn register_signals(builder: &init::ClassBuilder<Self>) {
        builder.add_signal(init::Signal {
            name: "player_list_changed",
            args: &[],
        });

        builder.add_signal(init::Signal {
            name: "connection_failed",
            args: &[],
        });

        builder.add_signal(init::Signal {
            name: "connection_succeeded",
            args: &[],
        });

        builder.add_signal(init::Signal {
            name: "game_ended",
            args: &[],
        });

        builder.add_signal(init::Signal {
            name: "game_error",
            args: &[init::SignalArgument {
                name: "what",
                default: Variant::from_i64(0),
                // some insight about hint and usage can be found in the "spinning_cube" tutorial
                hint: init::PropertyHint::None,
                usage: init::PropertyUsage::DEFAULT,
            }],
        });
    }

    fn _init(_owner: Node) -> Self {
        GameState {
            player_name: String::from("The Warrior"),
            players: HashMap::new(),
            players_ready: Vec::new(),
        }
    }

    #[export]
    unsafe fn _ready(&self, owner: Node) {
        let tree = &mut owner.get_tree().unwrap().cast::<Node>().unwrap();

        util::connect_signal(tree, &owner, "network_peer_connected", "player_connected");
        util::connect_signal(
            tree,
            &owner,
            "network_peer_disconnected",
            "player_disconnected",
        );
        util::connect_signal(tree, &owner, "connected_to_server", "connection_ok");
        util::connect_signal(tree, &owner, "connection_failed", "connection_failed");
        util::connect_signal(tree, &owner, "server_disconnected", "server_disconnected");

        godot_print!("__________________ GAMESTATE INITIATED");
    }

    #[export]
    unsafe fn player_connected(&self, mut owner: Node, id: i64) {
        owner.rpc_id(
            id,
            GodotString::from_str("register_player"),
            &[Variant::from_str(&self.player_name)],
        );
    }

    #[export]
    unsafe fn player_disconnected(&mut self, mut owner: Node, id: i64) {
        if owner.has_node(NodePath::from_str("/root/world")) {
            let tree = &owner.get_tree().unwrap();
            if tree.is_network_server() {
                let msg = format!("Player {} disconnected", id);
                owner.emit_signal(
                    GodotString::from_str("game_error"),
                    &[Variant::from_str(msg)],
                );
            }
            return;
        }
        self.unregister_player(owner, id);
    }

    // Callback from SceneTree, only for clients (not server)
    #[export]
    unsafe fn connection_ok(&self, mut owner: Node) {
        owner.emit_signal(GodotString::from_str("connection_succeeded"), &[]);
    }

    // Callback from SceneTree, only for clients (not server)
    #[export]
    unsafe fn connected_fail(&self, mut owner: Node) {
        let tree = &mut owner.get_tree().unwrap();
        tree.set_network_peer(Option::None);
        owner.emit_signal(GodotString::from_str("connection_failed"), &[]);
    }

    // Callback from SceneTree, only for clients (not server)
    #[export]
    unsafe fn server_disconnected(&mut self, mut owner: Node) {
        owner.emit_signal(
            GodotString::from_str("game_error"),
            &[Variant::from_str("Server disconnected")],
        );
        self.end_game(owner);
    }

    // Lobby management

    #[export]
    #[rpc(init::RpcMode::Remote)]
    fn register_player(&mut self, mut owner: Node, new_player_name: String) {
        let id = unsafe {
            let tree = &owner.get_tree().unwrap();
            tree.get_rpc_sender_id()
        };

        godot_print!("Player id '{}' connected", id);
        self.players.insert(id, new_player_name);

        unsafe {
            owner.emit_signal(GodotString::from_str("player_list_changed"), &[]);
        }
    }

    unsafe fn unregister_player(&mut self, mut owner: Node, id: i64) {
        self.players.remove(&id);
        owner.emit_signal(
            GodotString::from_str("player_list_changed"),
            &[Variant::from_i64(id)],
        );
    }

    unsafe fn end_game(&mut self, mut owner: Node) {
        if owner.has_node(NodePath::from_str("/root/world")) {
            owner
                .get_node(NodePath::from_str("/root/world"))
                .unwrap()
                .queue_free();
        }
        owner.emit_signal(GodotString::from_str("game_ended"), &[]);
        self.players.clear();

        let tree = &mut owner.get_tree().unwrap();
        tree.set_network_peer(None);
    }

    #[export]
    pub fn host_game(&mut self, owner: Node, new_player_name: String) {
        self.player_name = new_player_name;
        let mut host = NetworkedMultiplayerENet::new();
        host.create_server(DEFAULT_PORT, MAX_PEERS, 0, 0)
            .expect("Couldn't create server");
        unsafe {
            let tree = &mut owner.get_tree().unwrap();
            tree.set_network_peer(Some(host.to_networked_multiplayer_peer()));
        }
    }

    #[export]
    pub fn join_game(&mut self, owner: Node, ip: GodotString, new_player_name: String) {
        self.player_name = new_player_name;
        let mut host = NetworkedMultiplayerENet::new();
        host.create_client(ip, DEFAULT_PORT, 0, 0, 0)
            .expect("Couldn't create client");
        unsafe {
            let tree = &mut owner.get_tree().unwrap();
            tree.set_network_peer(Some(host.to_networked_multiplayer_peer()));
        }
    }

    pub fn get_player_list(&self) -> Vec<String> {
        let mut res = Vec::new();
        for (_, name) in self.players.iter() {
            res.push(name.clone());
        }
        return res;
    }

    pub fn get_player_name(&self) -> String {
        return self.player_name.clone();
    }

    #[export]
    pub fn begin_game(&self, mut owner: Node) {
        unsafe {
            let tree = &owner.get_tree().unwrap();
            assert!(tree.is_network_server());
        }

        let mut spawn_points = Dictionary::new();

        // Server in spawn point 0
        spawn_points.set(&Variant::from_i64(1), &Variant::from_i64(0));
        let mut spawn_point_idx: i64 = 1;
        for (id, _) in self.players.iter() {
            spawn_points.set(&Variant::from_i64(*id), &Variant::from_i64(spawn_point_idx));
            spawn_point_idx += 1;
        }

        for (id, _) in self.players.iter() {
            unsafe {
                owner.rpc_id(
                    *id,
                    GodotString::from_str("pre_start_game"),
                    &[Variant::from_dictionary(&spawn_points)],
                );
            }
        }

        unsafe {
            self.pre_start_game(owner, spawn_points);
        }
    }

    #[export]
    #[rpc(init::RpcMode::Remote)]
    unsafe fn pre_start_game(&self, mut owner: Node, spawn_points: Dictionary) {
        // Change scene
        let tree = &mut owner.get_tree().unwrap();

        let world_scene = util::load_scene("res://world.tscn").unwrap();
        let world = util::instance_scene::<Node2D>(&world_scene).ok().unwrap();

        tree.cast::<Node>()
            .unwrap()
            .add_child(Some(world.to_node()), false);

        util::get_node::<Control>(&owner, "/root/lobby")
            .unwrap()
            .hide();

        let player_scene = util::load_scene("res://player.tscn").unwrap();

        let network_id = tree.get_network_unique_id();
        for id in spawn_points.keys().iter() {
            let spawn_node_pth = format!("spawn_points/{}", spawn_points.get(id).to_string());
            let spawn_pos = util::get_node::<Position2D>(&owner, &spawn_node_pth)
                .unwrap()
                .get_position();

            let mut player = util::instance_scene::<KinematicBody2D>(&player_scene)
                .ok()
                .unwrap();
            //Use unique ID as node name
            player.set_name(GodotString::from_str(id.to_i64().to_string()));
            player.set_position(spawn_pos);
            //set unique id as master
            player.set_network_master(id.to_i64(), true);

            if id.to_i64() == network_id {
                // If node for this peer id, set name
                player.to_object().call(
                    GodotString::from_str("set_player_name"),
                    &[Variant::from_str(&self.player_name)],
                );
            } else {
                player.to_object().call(
                    GodotString::from_str("set_player_name"),
                    &[Variant::from_str(&self.players[&id.to_i64()])],
                );
            }

            world
                .get_node(NodePath::from_str("players"))
                .unwrap()
                .add_child(Some(player.to_node()), false);
        }

        //Set up score
        let mut score = world
            .get_node(NodePath::from_str("score"))
            .unwrap()
            .to_object();
        score.call(
            GodotString::from_str("add_player"),
            &[
                Variant::from_i64(network_id),
                Variant::from_str(&self.player_name),
            ],
        );
        for (id, name) in self.players.iter() {
            score.call(
                GodotString::from_str("add_player"),
                &[Variant::from_i64(id.clone()), Variant::from_str(&name)],
            );
        }

        unsafe {
            if !tree.is_network_server() {
                owner.rpc_id(
                    1,
                    GodotString::from_str("ready_to_start"),
                    &[Variant::from_i64(network_id)],
                );
            } else if self.players.is_empty() {
                self.post_start_game(owner);
            }
        }
        /*

        if not get_tree().is_network_server():
        # Tell server we are ready to start
        rpc_id(1, "ready_to_start", get_tree().get_network_unique_id())
        elif players.size() == 0:
            post_start_game()
        */
    }

    #[export]
    #[rpc(init::RpcMode::Remote)]
    unsafe fn post_start_game(&self, owner: Node) {
        //Unpause and unleash the game!
        let tree = &mut owner.get_tree().unwrap();
        tree.set_pause(false);
    }

    #[export]
    #[rpc(init::RpcMode::Remote)]
    fn ready_to_start(&mut self, mut owner: Node, player_id: i64) {
        unsafe {
            assert!(owner.get_tree().unwrap().is_network_server());
        }

        if !self.players_ready.contains(&player_id) {
            self.players_ready.push(player_id);
        }

        if self.players_ready.len() == self.players.len() {
            for (id, _) in self.players.iter() {
                unsafe {
                    owner.rpc_id(*id, GodotString::from_str("post_start_game"), &[]);
                    self.post_start_game(owner);
                }
            }
        }
    }
}
