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

  let circleFraction = function(gl, fraction) {
    let vertexCount = 40;
    let angleInc = 2 * Math.PI / vertexCount;
    let vertices = [];
    for (let i = 0; i < vertexCount * fraction; i++) {
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
      count: vertexCount * fraction * 3,
    };
  };

  let circle = function(gl) {
    return circleFraction(gl, 1);
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
    circleFraction: circleFraction,
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

  function stationShape(shapeName) {
    switch (shapeName) {
    case 'Circle':
      return glShapes.circle(gl);
    case 'Square':
      return glShapes.square(gl);
    case 'Triangle':
      return glShapes.triangle(gl);
    }
    return null;
  }

  function edge_thickness() {
    return game_model.state.station_size / 2.5;
  }

  function draw_stations() {
    let passengerSize = game_model.state.station_size / 2.5;
    let passengerMargin = 2;
    let station_size = game_model.state.station_size;
    let station_border_size = station_size / 5;

    for (let i = 0; i < game_model.state.stations.length; i++) {
      let station = game_model.state.stations[i];
      let station_type = station.t;
      let station_pos = station.position;
      let shape = stationShape(station_type);
      if (shape !== null) {
        let colour = [0, 0, 0];
        if (i === touched_station) {
          colour = [1, 0, 0];
        }
        if (station.blow_time > 0) {
          let fractionBlown = station.blow_time / game_model.state.time_to_blow;
          let blowShape = glShapes.circleFraction(gl, fractionBlown);
          glShapes.drawShape(gl, program, blowShape, station_pos, [0.4, 0.4, 0.4], station_size * 2, station_size * 2, 0);
        }
        glShapes.drawShape(gl, program, shape, station_pos, colour, station_size, station_size, 0);
        glShapes.drawShape(gl, program, shape, station_pos, [1, 1, 1], station_size - station_border_size, station_size - station_border_size, 0);
        for (let p = 0; p < station.passengers.length; p++) {
          let passenger = station.passengers[p];
          let passengersX = station_pos[0] + (station_size / 2) + (passengerSize / 2) + passengerMargin;
          let passengerX = passengersX + (p % 5) * (passengerSize + passengerMargin);
          let passengersY = station_pos[1] - (station_size / 2) + (passengerSize / 2);
          let passengerY = passengersY + Math.floor(p / 5) * (passengerSize + passengerMargin);
          let passengerPos = [ passengerX, passengerY];
          let shape = stationShape(passenger);
          if (shape !== null) {
            glShapes.drawShape(gl, program, shape, passengerPos, [0, 0, 0], passengerSize, passengerSize, 0);
          }
        }
      }
    }
  }

  function draw_edges(edgeList, colour) {
    let thickness = edge_thickness();
    for (let i = 0; i < edgeList.length; i++) {
      let edge = edgeList[i];
      let srcStn = game_model.state.stations[edge.origin];
      let tgtStn = game_model.state.stations[edge.destination];
      let via = edge.via_point;
      if (srcStn && tgtStn) {
        glShapes.drawLine(gl, program, srcStn.position[0], srcStn.position[1], via[0], via[1], thickness, colour);
        glShapes.drawLine(gl, program, via[0], via[1], tgtStn.position[0], tgtStn.position[1], thickness, colour);
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
    let passengerSize = game_model.state.station_size / 2.5;
    let passengerMargin = 1;
    let trainLength = 3 * passengerSize + 4 * passengerMargin;
    let trainWidth = 2 * passengerSize + 3 * passengerMargin;
    for (let i = 0; i < game_model.state.trains.length; i++) {
      let train = game_model.state.trains[i];
      let line = game_model.state.lines[train.on_line];
      if (!line) { continue; }
      let trainColour = line.colour;
      let trainX = train.position[0];
      let trainY = train.position[1];
      let headingX = train.heading[0];
      let headingY = train.heading[1];
      let travelX = headingX - trainX;
      let travelY = headingY - trainY;
      if (travelX === 0) {
        travelX = 1;
      }
      let travelAngle = Math.atan(travelY / travelX);
      glShapes.drawShape(gl, program, glShapes.square(gl), train.position, trainColour, trainLength, trainWidth, travelAngle);
      for (let p = 0; p < 6; p++) {
        let passenger = train.passengers[p];
        if (!passenger) {
          break;
        }
        let seatRow = p % 3;
        let seatCol = p % 2;
        let xMultiplier = passengerSize + passengerMargin;
        let yMultiplier = (passengerSize / 2) + passengerMargin;
        let passengerOffsetX = (seatRow - 1) * xMultiplier;
        let passengerOffsetY = seatCol === 0 ? -yMultiplier : yMultiplier;
        let r = Matrix.RotationZ(travelAngle).ensure4x4();
        let translate = Matrix.Translation($V([passengerOffsetX, passengerOffsetY, 0]));
        let passengerOffset = r.x(translate).col(4);
        let passengerPos = [
          train.position[0] + passengerOffset.e(1),
          train.position[1] + passengerOffset.e(2),
        ];
        let shape = stationShape(passenger);
        if (shape !== null) {
          glShapes.drawShape(gl, program, shape, passengerPos, [trainColour[0] + 0.1, trainColour[1] + 0.1, trainColour[2] + 0.1], passengerSize, passengerSize, travelAngle);
        }

      }
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

  function draw_player_lines() {
    let thickness = edge_thickness();
    let margin = thickness / 2;
    let top_margin = margin;
    let player_line_count = 0;
    for (let i = 0; i < game_model.state.lines.length; i++) {
      let line = game_model.state.lines[i];
      if (line.owning_player != this_player) {
        continue;
      }

      let y = (thickness / 2) + top_margin + (thickness + margin) * player_line_count;

      let p = getWorldCoords(10, y);
      let p2 = getWorldCoords(50, y);
      glShapes.drawLine(gl, program, p[0], p[1], p2[0], p2[1], thickness, line.colour);

      player_line_count++;
    }
  }

  function set_player_score() {
    if (typeof game_model.state.scores !== 'undefined') {
      let playerScore = game_model.state.scores[this_player] || 0;
      displayElements.score.innerHTML = 'Score: ' + playerScore;
      let canvasRect = displayElements.canvas.getBoundingClientRect();
      let canvasWidth = canvasRect.width;
      let canvasLeft = canvasRect.left;
      let canvasTop = canvasRect.top;
      let thisWidth = displayElements.score.getBoundingClientRect().width;
      let left = canvasWidth + canvasLeft - 30 - thisWidth;
      let top = canvasTop + 10;
      let styleString = 'position:absolute;left:' + left + 'px;' + 'top:' + top + 'px;';
      displayElements.score.style = styleString;
    }
  }

  function draw_HUD() {
    draw_player_lines();
    set_player_score();
  }

  function draw() {
    displayElements.count.innerText = game_model.lobby_count;
    if (!game_started) { return; }
    draw_state();
    draw_HUD();
  }

  function loop() {
    draw();
    window.requestAnimationFrame(loop);
  }

  function squareDistance(x1, y1, x2, y2) {
    let xDiff = x1 - x2;
    let yDiff = y1 - y2;
    return xDiff * xDiff + yDiff * yDiff;
  }

  function getClosestToWorldPoint(x, y) {
    let currentClosest = null;
    let currentSqrDist = null;
    for (let i = 0; i < game_model.state.stations.length; i++) {
      let station = game_model.state.stations[i];
      let sqrDist = squareDistance(x, y, station.position[0], station.position[1]);
      if (currentClosest === null || sqrDist < currentSqrDist) {
        currentClosest = i;
        currentSqrDist = sqrDist;
      }
    }
    return [currentClosest, currentSqrDist];
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
    if (typeof message.You !== 'undefined') {
      alert(message.You);
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

  function sendNewLine(startId, endId) {
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
    let localCopy = touched_station;  // Want to use a local copy and set to null now in case of early return
    touched_station = null;
    if (stationId !== localCopy) {
      for (let i = 0; i < game_model.state.lines.length; i++) {
        let line = game_model.state.lines[i];
        if (line.owning_player != this_player) { continue; }
        if (line.edges.length === 0) { continue; }
        if (line.edges[0].origin == localCopy) {
          sendInsertStation(i, stationId);
          return;
        }
        if (line.edges[line.edges.length - 1].destination == localCopy) {
          sendAppendStation(i, stationId);
          return;
        }
      }
      for (let i = 0; i < game_model.state.lines.length; i++) {
        let line = game_model.state.lines[i];
        if (line.owning_player != this_player) { continue; }
        if (line.edges.length !== 0) { continue; }
        sendNewLine(localCopy, stationId);
        return;
      }
    }
  }

  function handlePointerDown(x, y, pointerRadius) {
    if (!game_started) {
      sendStartGame();
    } else {
      let worldPointer = getWorldCoords(x, y);
      let worldOuter = getWorldCoords(x + pointerRadius, y);
      let closest = getClosestToWorldPoint(worldPointer[0], worldPointer[1]);
      let closestId = closest[0];
      let closestDistance = closest[1];
      let worldPointerRadius = squareDistance(worldPointer[0], worldPointer[1], worldOuter[0], worldOuter[1]);
      if (closestDistance <= worldPointerRadius) {
        handleStationDown(closestId);
      }
    }
  }

  function handlePointerUp(x, y, pointerRadius) {
    if (game_started) {
      let worldPointer = getWorldCoords(x, y);
      let worldOuter = getWorldCoords(x + pointerRadius, y);
      let closest = getClosestToWorldPoint(worldPointer[0], worldPointer[1]);
      let closestId = closest[0];
      let closestDistance = closest[1];
      let worldPointerRadius = squareDistance(worldPointer[0], worldPointer[1], worldOuter[0], worldOuter[1]);
      if (closestDistance <= worldPointerRadius) {
        handleStationUp(closestId);
      }
    }
  }

  function attachInputs() {
    window.addEventListener('touchstart', function(e) {
      let touchPoint = e.touches[0];
      let bounding = displayElements.canvas.getBoundingClientRect();
      let x = touchPoint.pageX - bounding.x;
      let y = touchPoint.pageY - bounding.y;
      handlePointerDown(x, y, displayElements.canvas.width);
    });
    window.addEventListener('touchend', function(e) {
      let touchPoint = e.changedTouches[0];
      let bounding = displayElements.canvas.getBoundingClientRect();
      let x = touchPoint.pageX - bounding.x;
      let y = touchPoint.pageY - bounding.y;
      handlePointerUp(x, y, displayElements.canvas.width);
    });
    window.addEventListener('mousedown', function(e) {
      let bounding = displayElements.canvas.getBoundingClientRect();
      let x = e.clientX - bounding.x;
      let y = e.clientY - bounding.y;
      handlePointerDown(x, y, 1);
    });
    window.addEventListener('mouseup', function(e) {
      let bounding = displayElements.canvas.getBoundingClientRect();
      let x = e.clientX - bounding.x;
      let y = e.clientY - bounding.y;
      handlePointerUp(x, y, 1);
    });
  }

  function setup(websocketAddress, gameEl, lobbyEl, statusEl, countEl, canvasEl, scoreEl) {
    hideElement(gameEl);
    showElement(lobbyEl);
    displayElements.status = statusEl;
    displayElements.lobby = lobbyEl;
    displayElements.canvas = canvasEl;
    displayElements.count = countEl;
    displayElements.game = gameEl;
    displayElements.score = scoreEl;
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
