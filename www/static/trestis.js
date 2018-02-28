/*global log makeShaderProgram $V DegToRad Matrix SetUpAttributes makeOrtho runApp*/

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

function initBuffers(gl) {
  const vertices = [
    0, 0,
    0, 1,
    1, 0,
    1, 1,
    1, 0,
    0, 1
  ];
  let VBO = gl.createBuffer();
  gl.bindBuffer(gl.ARRAY_BUFFER, VBO);
  gl.bufferData(gl.ARRAY_BUFFER, new Float32Array(vertices), gl.STATIC_DRAW);
  return VBO;
}

function App(gl, keys) {
  log('App', 1);
  const self = this;

  const program = makeShaderProgram(gl, vertexShaderSource, fragmentShaderSource);
  const scoreTable = document.getElementById('scoreTable');

  initBuffers(gl);
  let playerPoses = {};
  let playerScores = {};
  let playerNames = {};
  let grid = [[]];

  function connect(address) {
    if (self.ws) {
      log('Already have a ws', 0);
      return;
    }
    log('attempting connect to ' + address, 1);
    let ws = new WebSocket('ws://' + address + ':12345');
    ws.onopen = function() {
      self.ws = ws;
      self.loop();
    };
    ws.onmessage = function(m) {
      log(m.data, 3);
      self.inputEvents.push(JSON.parse(m.data));
    };
    ws.onerror = function(e) {
      log('websocket error: ' + e, 1);
      self.loop = function() {};
      self.ws = null;
    };
  }
  document
    .getElementById('connectButton')
    .addEventListener('click', function() {
      connect(document.getElementById('connectBox').value);
    });
  document
    .getElementById('connectBox')
    .value = window.location.hostname;
  document
    .getElementById('nameBox')
    .addEventListener('change', function(n) {
      let newName = n.target.value;
      if (newName) {
        self.desiredNameChange = newName;
      }
    });
  self.desiredNameChange = null;

  self.inputEvents = [];
  let lastTime = new Date().getTime();
  self.loop = function() {
    if (!self.ws) {
      log('not connected', 1);
      return;
    }
    let currentTime = new Date().getTime();
    let delta = currentTime - lastTime;
    self.handleInput(delta);
    self.draw(delta);
    lastTime = currentTime;
    requestAnimationFrame(self.loop);
  };

  let pieceColour = function(piece) {
    switch (piece) {
    case 'T':
      return [0.5, 0, 0.5 ];
    case 'L':
      return [1, 0.6, 0 ];
    case 'J':
      return [0, 0, 1 ];
    case 'S':
      return [0, 1, 0 ];
    case 'Z':
      return [1, 0, 0 ];
    case 'I':
      return [0, 1, 1 ];
    case 'O':
      return [1, 1, 0 ];
    }
  };

  let pieceSquares = function(piece) {
    switch (piece) {
    case 'T':
      return [
        $V([0, 2, 0]),
        $V([1, 1, 0]),
        $V([1, 2, 0]),
        $V([2, 2, 0])
      ];
    case 'L':
      return [
        $V([0, 1, 0]),
        $V([1, 1, 0]),
        $V([2, 1, 0]),
        $V([2, 0, 0])
      ];
    case 'J':
      return [
        $V([0, 1, 0]),
        $V([1, 1, 0]),
        $V([2, 1, 0]),
        $V([2, 2, 0])
      ];
    case 'S':
      return [
        $V([1, 2, 0]),
        $V([2, 1, 0]),
        $V([2, 2, 0]),
        $V([3, 1, 0])
      ];
    case 'Z':
      return [
        $V([0, 1, 0]),
        $V([1, 1, 0]),
        $V([1, 2, 0]),
        $V([2, 2, 0])
      ];
    case 'I':
      return [
        $V([0, 1, 0]),
        $V([1, 1, 0]),
        $V([2, 1, 0]),
        $V([3, 1, 0])
      ];
    case 'O':
      return [
        $V([1, 1, 0]),
        $V([1, 2, 0]),
        $V([2, 2, 0]),
        $V([2, 1, 0])
      ];
    }
    return [];
  };
  let rotationMatrix = function(rotation) {
    let angle = rotation * DegToRad(90);
    let translateToCentre = Matrix.Translation($V([-2, -2, 0])).ensure4x4();
    let actualRotation = Matrix.RotationZ(angle).ensure4x4();
    let translateBack = Matrix.Translation($V([2, 2, 0])).ensure4x4();

    let moveAndRotate = actualRotation.x(translateToCentre);
    let rotateAndMove = translateBack.x(moveAndRotate);
    return rotateAndMove;
  };
  let drawPiece = function(pieceType, pieceRotation, localOrigin, alpha) {
    let colour = pieceColour(pieceType);
    program.setUniformVec4('colour', colour[0], colour[1], colour[2], alpha);

    log(pieceType + ' at ' + localOrigin.elements, 3);
    let squarePoses = pieceSquares(pieceType);
    for (var piecePosI in squarePoses) {
      let piecePos = squarePoses[piecePosI];
      let rotation = rotationMatrix(pieceRotation);
      let model = localOrigin.x(rotation.x(Matrix.Translation(piecePos)));
      program.setUniformMat4('model', model);

      gl.drawArrays(gl.TRIANGLES, 0, 6);
    }
  };
  let drawPlayer = function(player, gridPos) {
    let pos = playerPoses[player];
    if (!pos) {
      return;
    }
    let alpha = 1;
    if (player != self.playerId) {
      alpha = 0.35;
    }
    let m = Matrix.Translation($V([pos.x, pos.y, 0]));
    let localOrigin = m.x(Matrix.Translation(gridPos));
    drawPiece(pos.t, pos.r, localOrigin, alpha);
  };
  let drawSelf = function(gridPos) {
    let pos = playerPoses[self.playerId];
    if (!pos) {
      return;
    }
    drawPlayer(self.playerId, gridPos);

    let nextPieceBgPos = Matrix.Translation($V([1, 1, 0]));
    let nextPieceBgSize = Matrix.Diagonal([4, 4, 0, 1]);
    let m = nextPieceBgPos.x(nextPieceBgSize);

    program.setUniformMat4('model', m);
    program.setUniformVec4('colour', 1, 1, 1, 1);
    gl.drawArrays(gl.TRIANGLES, 0, 6);
    let next = pos.next_t;
    let nextPiecePos = Matrix.Translation($V([1, 1, 0]));
    drawPiece(next, 0, nextPiecePos, 1);
  };

  self.draw = function() {
    gl.clearColor(0.2, 0.3, 0.3, 1.0);
    gl.clear(gl.COLOR_BUFFER_BIT);

    program.use();
    SetUpAttributes(gl, program, [['aPos', 2]], 4);

    let ortho = makeOrtho(0, 23, 23, 0, -1, 1);
    program.setUniformMat4('projection', ortho);

    let gridPos = $V([6, 0, 0]);

    if (grid && grid[0]) {
      let height = grid.length;
      let width = grid[0].length;

      let scale = Matrix.Diagonal([width, height, 0, 1]);
      let m = Matrix.Translation(gridPos).x(scale);

      program.setUniformMat4('model', m);
      program.setUniformVec4('colour', 1, 1, 1, 1);
      gl.drawArrays(gl.TRIANGLES, 0, 6);
    }

    for (var y = 0; y < grid.length; y++) {
      let row = grid[y];
      for (var x = 0; x < row.length; x++) {
        if (!row[x]) { continue; }
        let m = Matrix.Translation($V([x, y, 0]));
        program.setUniformMat4('model', m.x(Matrix.Translation(gridPos)));
        let colour = pieceColour(row[x]);
        program.setUniformVec4('colour', colour[0], colour[1], colour[2], 1);

        gl.drawArrays(gl.TRIANGLES, 0, 6);
      }
    }

    for (var player in playerPoses) {
      if (player == self.playerId) {
        continue;
      }
      drawPlayer(player, gridPos);
    }
    drawSelf(gridPos);
  };

  const updatePlayerScore = function(id) {
    let elId = 'score' + id;
    let playerEntry = document.getElementById(elId);
    if (playerEntry === null) {
      playerEntry = document.createElement('p');
      playerEntry.id = elId;
      scoreTable.append(playerEntry);
    }
    let playerName = playerNames[id] || ('Player ' + id);
    let playerScore = playerScores[id] || 0;
    playerEntry.innerText =  playerName + ': ' + playerScore;
  };

  function sendMoveAction(actionType) {
    self.ws.send(JSON.stringify({ 'Action': actionType }));
  }

  const controls = function(k) {
    let keyLeft = k.isKeyDown(k.A);
    let keyRight = k.isKeyDown(k.D);
    let keyUp = k.isKeyDown(k.W);
    let keyDown = k.isKeyDown(k.S);

    let t = k.getTouch();
    // Get it a good way
    let viewport = [0, 0, 900, 810];
    let centre_x = viewport[2] / 2;
    let centre_y = viewport[3] / 2;

    let touchLeft = false;
    let touchRight = false;
    let touchUp = false;
    let touchDown = false;
    if (t) {
      let tXDist = t.x - centre_x;
      let tYDist = t.y - centre_y;
      if (Math.abs(tXDist) > Math.abs(tYDist)) {
        if (tXDist > 0) {
          touchRight = true;
        } else {
          touchLeft = true;
        }
      } else {
        if (tYDist > 0) {
          touchDown = true;
        } else {
          touchUp = true;
        }
      }
    }

    return {
      left: keyLeft || touchLeft,
      right: keyRight || touchRight,
      up: keyUp || touchUp,
      down: keyDown || touchDown
    };
  };

  self.lastControls = {
    left: null,
    up: null,
    down: null,
    right: null,
  };

  function handleControl(controls, delta, controlName, actionType) {
    const holdKeyDelay = 800;
    const holdKeyRepeat = 500;
    if (controls[controlName]) {
      if (self.lastControls[controlName] === null || self.lastControls[controlName] <= 0) {
        sendMoveAction(actionType);
      }
      if (self.lastControls[controlName] === null) {
        self.lastControls[controlName] = holdKeyDelay;
      } else if (self.lastControls[controlName] <= 0) {
        self.lastControls[controlName] = holdKeyRepeat;
      }
      self.lastControls[controlName] -= delta;
    } else {
      self.lastControls[controlName] = null;
    }
  }
  self.playerId = 0;
  self.handleInput = function(delta) {
    let c = controls(keys);
    handleControl(c, delta, 'left', 'Left');
    handleControl(c, delta, 'right', 'Right');
    handleControl(c, delta, 'up', 'Rotate');
    handleControl(c, delta, 'down', 'Down');
    if (self.desiredNameChange) {
      self.ws.send(JSON.stringify({ 'Name': self.desiredNameChange }));
      self.desiredNameChange = null;
    }
    for (var i = 0; i < self.inputEvents.length; i++) {
      let ev = self.inputEvents.shift();
      if (ev.Text) {
        log(ev.Text, 1);
      }
      if (ev.Connected) {
        self.playerId = ev.Connected;
        log('I am ' + self.playerId, 2);
      }
      if (ev.StateUpdate) {
        playerPoses[ev.StateUpdate[0]] = ev.StateUpdate[1];
        log('StateUpdate for ' + ev.StateUpdate[0] + ': ' + ev.StateUpdate[1], 2);
      }
      if (ev.Disconnection) {
        let discon_id = ev.Disconnection;
        delete playerPoses[discon_id];
        delete playerScores[discon_id];
        delete playerNames[discon_id];
        log(discon_id + ' disconnected!', 2);
      }
      if (ev.GridUpdate) {
        grid = ev.GridUpdate.cells;
        log('Grid: ' + grid, 2);
      }
      if (ev.PointsUpdate) {
        const playerId = ev.PointsUpdate[0];
        const score = ev.PointsUpdate[1];
        log('Player ' + playerId + ': ' + score + ' points!', 1);
        playerScores[playerId] = score;
        updatePlayerScore(playerId);
      }
      if (ev.NameUpdate) {
        const playerId = ev.NameUpdate[0];
        const name = ev.NameUpdate[1];
        log('Player ' + playerId + ' is called ' + name, 1);
        playerNames[playerId] = name;
        updatePlayerScore(playerId);
        if (playerId === self.playerId) {
          let namebox = document.getElementById('nameBox');
          if (self.desiredNameChange == null) {
            namebox.value = name;
          }
        }
      }
    }
  };
}

window.onload = function() {
  log('running', 1);
  let canvas = document.getElementsByTagName('canvas')[0];
  log('canvas ' + canvas, 1);
  runApp(App, canvas);
};

window.onerror = function(message, url, lineNumber) {
  log('' + lineNumber + ': ' + message, 0);
  return true;
};

