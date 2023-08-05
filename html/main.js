const socket = new WebSocket("ws://localhost:3000");

// Listen for messages
socket.addEventListener("message", (event) => {
    if(event.data == "tick") {
        return;
    }
    
    console.log("Message from server ", event.data);
});
