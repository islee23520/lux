extends EditorPlugin

var tcp_client: StreamPeerTCP

func _enter_tree():
	tcp_client = StreamPeerTCP.new()
	tcp_client.connect_to_host("127.0.0.1", 17342)

func _exit_tree():
	if tcp_client:
		tcp_client.disconnect_from_host()
