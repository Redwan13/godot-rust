use gdnative::*;

#[derive(NativeClass)]
#[inherit(Area2D)]
pub struct Bomb {
    in_area: Vec<i32>,
    //from_player: Option<Node>,
}

#[methods]
impl Bomb {
    fn _init(_owner: Area2D) -> Self {
        Bomb {
            in_area: Vec::new(),
            // from_player: Option::None,
        }
    }

    #[export]
    unsafe fn explode(&self, owner: Area2D) {
        if !owner.is_network_master() {
            return;
        }

        //for p in self.in_area {
        //            if p.has_method("exploded") {
        //                //Exploded rpc mode is RpcMode::Master, so it will only be received by the master
        //                p.rpc("exploded"); //TODO : Pass bomb owner
        //            }
        //}
    }

    #[export]
    fn done(&self, mut owner: Area2D) {
        unsafe {
            owner.queue_free();
        }
    }
}
/*
extends Area2D

var in_area = []
var from_player

# Called from the animation
func explode():
    if not is_network_master():
        # But will call explosion only on master
        return
    for p in in_area:
        if p.has_method("exploded"):
            p.rpc("exploded", from_player) # Exploded has a master keyword, so it will only be received by the master

func done():
    queue_free()

func _on_bomb_body_enter(body):
    if not body in in_area:
        in_area.append(body)

func _on_bomb_body_exit(body):
    in_area.erase(body)

*/
