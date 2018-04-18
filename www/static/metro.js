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
  let drawShape = function(gl, program, shape, pos, colour, sizeX, sizeY, rotation) {
    program.setUniformVec4('colour', colour[0], colour[1], colour[2], 1.0);
    let scale = Matrix.Diagonal([sizeX, sizeY, 0, 1]);
    let m = scale;

    let r = Matrix.RotationZ(rotation).ensure4x4();
    m = r.x(m);

    let translate = Matrix.Translation($V([pos[0], pos[1], 0]));
    m = translate.x(m);
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

  let triangle = function(gl) {
    const vertices = [
      -0.5, 0.5,
      0, -0.5,
      0.5, 0.5,
    ];
    return {
      vertices: bufferFromVertices(gl, vertices),
      count: 3,
    };
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

  let drawLine = function(gl, program, startX, startY, endX, endY, thickness, colour) {
    let midX = (startX + endX) / 2;
    let midY = (startY + endY) / 2;
    let dY = endY - startY;
    let dX = endX - startX;
    let angle = Math.atan(dY / dX);
    let distance = Math.sqrt((dY * dY) + (dX * dX));

    drawShape(gl, program, square(gl), [midX, midY], colour, distance, thickness, angle);
  };

  return {
    triangle: triangle,
    square: square,
    circle: circle,
    drawShape: drawShape,
    drawLine: drawLine,
  };
})();

let metro = (function() {
  let game_started = false;
  let game_model = {
    lobby_count: 0,
  };
  let this_player = null;
  let touched_station = null;

  let displayElements = {};
  function hideElement(el) { el.style.display = 'none'; }
  function showElement(el) { el.style.display = 'initial'; }
  let gl = null;
  let program = null;

  function getProjectionMatrix() {
    let canvasWidth = displayElements.canvas.width;
    let canvasHeight = displayElements.canvas.height;
    return makeOrtho(-(canvasWidth / 2), (canvasWidth / 2)
      , (canvasHeight / 2), -(canvasHeight / 2)
      , -1, 1);
  }

  function draw_stations() {
    let station_size = game_model.state.station_size;
    let station_border_size = station_size / 5;

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
      case 'Triangle':
        shape = glShapes.triangle(gl);
        break;
      }
      if (shape !== null) {
        let colour = [0, 0, 0];
        if (i === touched_station) {
          colour = [1, 0, 0];
        }
        glShapes.drawShape(gl, program, shape, station_pos, colour, station_size, station_size, 0);
        glShapes.drawShape(gl, program, shape, station_pos, [1, 1, 1], station_size - station_border_size, station_size - station_border_size, 0);
      }
    }
  }

  function draw_edges(edgeList, colour) {
    let edge_thickness = 8;
    for (let i = 0; i < edgeList.length; i++) {
      let edge = edgeList[i];
      let srcStn = game_model.state.stations[edge.origin];
      let tgtStn = game_model.state.stations[edge.destination];
      let via = edge.via_point;
      if (srcStn && tgtStn) {
        glShapes.drawLine(gl, program, srcStn.position[0], srcStn.position[1], via[0], via[1], edge_thickness, colour);
        glShapes.drawLine(gl, program, via[0], via[1], tgtStn.position[0], tgtStn.position[1], edge_thickness, colour);
      }
    }
  }
  function draw_lines() {
    for (let i = 0; i < game_model.state.lines.length; i++) {
      let line = game_model.state.lines[i];
      draw_edges(line.edges, line.colour);
    }
  }

  function draw_trains() {
    let trainLength = game_model.state.station_size + 10;
    let trainWidth = game_model.state.station_size - 5;
    for (let i = 0; i < game_model.state.trains.length; i++) {
      let train = game_model.state.trains[i];
      let trainX = train.position[0];
      let trainY = train.position[1];
      let headingX = train.heading[0];
      let headingY = train.heading[1];
      let travelX = headingX - trainX;
      let travelY = headingY - trainY;
      let travelAngle = Math.atan(travelY / travelX);
      glShapes.drawShape(gl, program, glShapes.square(gl), train.position, [1, 0, 1], trainLength, trainWidth, travelAngle);
    }
  }

  function draw_state() {
    gl.clearColor(0.945, 0.941, 0.922, 1.0);
    gl.clear(gl.COLOR_BUFFER_BIT);
    program.use();
    let ortho = getProjectionMatrix();
    program.setUniformMat4('projection', ortho);
    draw_lines();
    draw_trains();
    draw_stations();
  }

  function draw() {
    displayElements.count.innerText = game_model.lobby_count;
    if (!game_started) { return; }
    draw_state();
  }

  function loop() {
    draw();
    window.requestAnimationFrame(loop);
  }

  function canvasPointToGlPoint(canvasX, canvasY) {
    let halfCanvasWidth = displayElements.canvas.width / 2;
    let halfCanvasHeight = displayElements.canvas.height / 2;
    let centeredX = canvasX - halfCanvasWidth;
    let centeredY = canvasY - halfCanvasHeight;
    let glX = centeredX / halfCanvasWidth;
    let glY = -(centeredY / halfCanvasHeight);
    return [glX, glY];
  }

  function getWorldCoords(canvasX, canvasY) {
    let glPoint = canvasPointToGlPoint(canvasX, canvasY);
    let projection = getProjectionMatrix();
    let unProjection = projection.inverse();
    if (unProjection === null) {
      return [canvasX, canvasY];
    }
    let v = $V([glPoint[0], glPoint[1], 0, 1]).toDiagonalMatrix();
    let unprojected = v.x(unProjection);
    let worldX = unprojected.e(1, 1);
    let worldY = unprojected.e(2, 2);
    return [worldX, worldY];
  }

  function getStationAtScreenPoint(canvasX, canvasY) {
    let worldPoint = getWorldCoords(canvasX, canvasY);
    let x = worldPoint[0];
    let y = worldPoint[1];
    for (let i = 0; i < game_model.state.stations.length; i++) {
      let station = game_model.state.stations[i];
      let stationRight = station.position[0] + game_model.state.station_size / 2;
      let stationLeft = station.position[0] - game_model.state.station_size / 2;
      let stationTop = station.position[1] - game_model.state.station_size / 2;
      let stationBottom = station.position[1] + game_model.state.station_size / 2;
      if (x > stationRight) { continue; }
      if (x < stationLeft) { continue; }
      if (y > stationBottom) { continue; }
      if (y < stationTop) { continue; }
      return i;
    }
    return null;
  }

  let ws = null;
  function sendWebSocketMessage(obj) {
    if (!ws) { return; }
    ws.send(JSON.stringify(obj));
  }

  function handleWebSocketMessage(message) {
    if (message.LobbyCount) {
      game_model.lobby_count = message.LobbyCount;
    }
    if (message.GameState) {
      game_model.state = message.GameState;
      if (!game_started) {
        showElement(displayElements.game);
        hideElement(displayElements.lobby);
        game_started = true;
      }
    }
    if (message.You) {
      this_player = message.You;
    }
  }

  function setupWebSocket(address) {
    ws = new WebSocket(address);
    displayElements.status.innerText = 'Connecting';
    ws.onopen = function() {
      displayElements.status.innerText = 'Connected';
    };
    ws.onmessage = function(m) {
      handleWebSocketMessage(JSON.parse(m.data));
    };
    ws.onerror = function(m) {
      alert(JSON.stringify(m));
    };
  }

  function sendAddConnection(startId, endId) {
    if (startId === null || endId === null) { return; }
    sendWebSocketMessage({ NewLine: [ startId, endId ] });
  }
  function sendInsertStation(lineId, stationId) {
    if (lineId === null || stationId === null) { return; }
    sendWebSocketMessage({ InsertAtLineBeginning: [ lineId, stationId ] });
  }
  function sendAppendStation(lineId, stationId) {
    if (lineId === null || stationId === null) { return; }
    sendWebSocketMessage({ InsertAtLineEnd: [ lineId, stationId ] });
  }
  function sendStartGame() {
    sendWebSocketMessage({ StartGame: null });
  }

  function handleStationDown(stationId) {
    touched_station = stationId;
  }

  function handleStationUp(stationId) {
    if (touched_station === null) { return; }
    if (stationId !== touched_station) {
      for (let i = 0; i < game_model.state.lines.length; i++) {
        let line = game_model.state.lines[i];
        if (line.owning_player == this_player) {
          if (line.edges.length === 0) { continue; }
          if (line.edges[0].origin == touched_station) {
            sendInsertStation(i, stationId);
            return;
          }
          if (line.edges[line.edges.length() - 1].destination == touched_station) {
            sendAppendStation(i, stationId);
            return;
          }
        }
        // Server will validate this one :)
        sendAddConnection(touched_station, stationId);
      }
    }
    touched_station = null;
  }

  function handlePointerDown(x, y) {
    if (!game_started) {
      sendStartGame();
    } else {
      handleStationDown(getStationAtScreenPoint(x, y));
    }
  }

  function handlePointerUp(x, y) {
    if (game_started) {
      handleStationUp(getStationAtScreenPoint(x, y));
    }
  }

  function attachInputs() {
    window.addEventListener('touchstart', function(e) {
      let touchPoint = e.touches[0];
      let bounding = displayElements.canvas.getBoundingClientRect();
      let x = touchPoint.pageX - bounding.x;
      let y = touchPoint.pageY - bounding.y;
      handlePointerDown(x, y);
    });
    window.addEventListener('touchend', function(e) {
      let touchPoint = e.changedTouches[0];
      let bounding = displayElements.canvas.getBoundingClientRect();
      let x = touchPoint.pageX - bounding.x;
      let y = touchPoint.pageY - bounding.y;
      handlePointerUp(x, y);
    });
    window.addEventListener('mousedown', function(e) {
      let bounding = displayElements.canvas.getBoundingClientRect();
      let x = e.clientX - bounding.x;
      let y = e.clientY - bounding.y;
      handlePointerDown(x, y);
    });
    window.addEventListener('mouseup', function(e) {
      let bounding = displayElements.canvas.getBoundingClientRect();
      let x = e.clientX - bounding.x;
      let y = e.clientY - bounding.y;
      handlePointerUp(x, y);
    });
  }

  function setup(websocketAddress, gameEl, lobbyEl, statusEl, countEl, canvasEl) {
    hideElement(gameEl);
    showElement(lobbyEl);
    displayElements.status = statusEl;
    displayElements.lobby = lobbyEl;
    displayElements.canvas = canvasEl;
    displayElements.count = countEl;
    displayElements.game = gameEl;
    canvasEl.width = document.body.clientWidth;
    canvasEl.height = document.body.clientHeight;

    setupWebSocket(websocketAddress);

    gl = canvasEl.getContext('webgl');
    program = makeShaderProgram(gl, vertexShaderSource, fragmentShaderSource);
    attachInputs();
  }

  function start() {
    window.requestAnimationFrame(loop);
  }

  return {
    setup: setup,
    start: start,
  };
})();
