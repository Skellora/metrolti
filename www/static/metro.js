let metro = (function() {
  let game_started = false;
  let game_model = {
    lobby_count: 0,
  }

  let displayElements = {};
  function hideElement(el) { el.style.display = 'none'; }
  function showElement(el) { el.style.display = 'initial'; }

  function draw() {
    displayElements.lobby.innerText = game_model.lobby_count;
  }

  function loop() {
    draw();
    window.requestAnimationFrame(loop);
  }

  function handleWebSocketMessage(jsonM) {
    if (jsonM.LobbyCount) {
      game_model.lobby_count = jsonM.LobbyCount;
    }
  }

  let ws = null;
  function setup(websocketAddress, statusEl, lobbyEl, canvasEl) {
    hideElement(canvasEl);
    showElement(lobbyEl);
    displayElements.status = statusEl;
    displayElements.lobby = lobbyEl;
    displayElements.canvas = canvasEl;

    ws = new WebSocket(websocketAddress);
    statusEl.innerText = 'Connecting';
    ws.onopen = function() {
      statusEl.innerText = 'Connected';
    }
    ws.onmessage = function(m) {
      handleWebSocketMessage(JSON.parse(m.data));
    };
    ws.onerror = function(m) {
      alert(JSON.stringify(m));
    };
  }

  function start() {
    window.requestAnimationFrame(loop);
  }

  return {
    setup: setup,
    start: start,
  };
})();
