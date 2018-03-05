/* global makeShaderProgram SetUpAttributes makeOrtho Matrix $V */
const vertexShaderSource = `
attribute vec2 aPos;

uniform mat4 model;
uniform mat4 projection;

void main() {
	gl_Position = projection * model  * vec4(aPos, 0.0, 1.0);
}
`;
const fragmentShaderSource = `
uniform lowp vec4 colour;

void main() {
	gl_FragColor = colour;
}
`;

let glShapes = (function() {
  let drawShape = function(gl, program, shape, pos, colour, size) {
    program.setUniformVec4('colour', colour[0], colour[1], colour[2], 1.0);
    let scale = Matrix.Diagonal([size, size, 0, 1]);
    let m = Matrix.Translation($V([pos[0], pos[1], 0])).x(scale);
    program.setUniformMat4('model', m);
    gl.bindBuffer(gl.ARRAY_BUFFER, shape.vertices);
    SetUpAttributes(gl, program, [['aPos', 2]], 4);
    gl.drawArrays(gl.TRIANGLES, 0, shape.count);
  };

  let bufferFromVertices = function(gl, vertices) {
    let VBO = gl.createBuffer();
    gl.bindBuffer(gl.ARRAY_BUFFER, VBO);
    gl.bufferData(gl.ARRAY_BUFFER, new Float32Array(vertices), gl.STATIC_DRAW);
    return VBO;
  };

  let square = function(gl) {
    const vertices = [
      -0.5, -0.5,
      -0.5, 0.5,
      0.5, -0.5,
      0.5, 0.5,
      0.5, -0.5,
      -0.5,  0.5
    ];
    return {
      vertices: bufferFromVertices(gl, vertices),
      count: 6,
    };
  };

  let circle = function(gl) {
    let vertexCount = 40;
    let angleInc = 2 * Math.PI / vertexCount;
    let vertices = [];
    for (let i = 0; i < vertexCount; i++) {
      vertices.push(0);
      vertices.push(0);
      let x = Math.cos(i * angleInc) * 0.5;
      let y = Math.sin(i * angleInc) * 0.5;
      vertices.push(x);
      vertices.push(y);
      let x2 = Math.cos((i + 1) * angleInc) * 0.5;
      let y2 = Math.sin((i + 1) * angleInc) * 0.5;
      vertices.push(x2);
      vertices.push(y2);
    }
    return {
      vertices: bufferFromVertices(gl, vertices),
      count: vertexCount * 3,
    };
  };

  return {
    square: square,
    circle: circle,
    drawShape: drawShape,
  };
})();

let metro = (function() {
  let game_started = false;
  let game_model = {
    lobby_count: 0,
  };

  let displayElements = {};
  function hideElement(el) { el.style.display = 'none'; }
  function showElement(el) { el.style.display = 'initial'; }
  let gl = null;
  let program = null;

  function draw_state() {
    gl.clearColor(0.2, 0.3, 0.3, 1.0);
    gl.clear(gl.COLOR_BUFFER_BIT);
    program.use();
    let canvasWidth = displayElements.canvas.width;
    let canvasHeight = displayElements.canvas.height;
    let ortho =
      makeOrtho(-(canvasWidth / 2), (canvasWidth / 2)
        , (canvasHeight / 2), -(canvasHeight / 2)
        , -1, 1);
    program.setUniformMat4('projection', ortho);

    for (let i = 0; i < game_model.state.stations.length; i++) {
      let station = game_model.state.stations[i];
      let station_type = station.t;
      let station_pos = station.position;
      let shape = null;
      switch (station_type) {
      case 'Circle':
        shape = glShapes.circle(gl);
        break;
      case 'Square':
        shape = glShapes.square(gl);
        break;
      }
      if (shape !== null) {
        glShapes.drawShape(gl, program, shape, station_pos, [0, 0, 0], 50);
        glShapes.drawShape(gl, program, shape, station_pos, [1, 1, 1], 40);
      }
    }
  }

  function draw() {
    displayElements.lobby.innerText = game_model.lobby_count;
    if (!game_started) { return; }
    draw_state();
  }

  function loop() {
    draw();
    window.requestAnimationFrame(loop);
  }

  function handleWebSocketMessage(jsonM) {
    if (jsonM.LobbyCount) {
      game_model.lobby_count = jsonM.LobbyCount;
    }
    if (jsonM.GameState) {
      game_model.state = jsonM.GameState;
      if (!game_started) {
        showElement(displayElements.canvas);
        alert(JSON.stringify(jsonM.GameState));
        game_started = true;
      }
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
    };
    ws.onmessage = function(m) {
      handleWebSocketMessage(JSON.parse(m.data));
    };
    ws.onerror = function(m) {
      alert(JSON.stringify(m));
    };

    gl = canvasEl.getContext('webgl');
    program = makeShaderProgram(gl, vertexShaderSource, fragmentShaderSource);
    window.addEventListener('touchstart', function() {
      if (!game_started) {
        ws.send('{ "StartGame": null }');
      }
    });
  }

  function start() {
    window.requestAnimationFrame(loop);
  }

  return {
    setup: setup,
    start: start,
  };
})();
