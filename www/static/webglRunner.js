//
// MAIN
//
function runApp(appC, canvas) {
  if (!canvas) {
    log('Could not find canvas.', 1);
    return;
  }
  // Initialize the GL context
  const gl = canvas.getContext('webgl');

  // Only continue if WebGL is available and working
  if (!gl) {
    log('Unable to initialize WebGL. Your browser or machine may not support it.', 1);
    return;
  }

  log('gl initialised', 1);

  if (!appC) {
    log('Unable to find App', 0);
    return;
  }

  let Keys = {
    _pressed: {},
    _touch: null,
    _mouse: null,

    A: 65,
    D: 68,
    S: 83,
    W: 87,

    isKeyDown: function(key) {
      return this._pressed[key] == true;
    },

    keyDown: function(key) {
      log(key + ' down', 2);
      this._pressed[key] = true;
    },

    keyUp: function(key) {
      delete this._pressed[key];
    },

    touchPos: function(touchX, touchY) {
      this._touch = { x: touchX, y: touchY };
    },

    unTouch: function() {
      this._touch = null;
    },

    getTouch: function() {
      return this._touch;
    },

    mousePos: function(mouseX, mouseY) {
      this._mouse = { x: mouseX, y: mouseY };
    },

    getPointer: function () {
      if (this._mouse) {
        return this._mouse;
      }
      return this._touch;
    }
  };

  window.addEventListener('keyup', function(event) { Keys.keyUp(event.keyCode); }, false);
  window.addEventListener('keydown', function(event) { Keys.keyDown(event.keyCode); }, false);

  if (!canvas.requestPointerLock) {
    canvas.addEventListener('mousemove', function(event) { Keys.mousePos(event.clientX, event.clientY); }, false);
  } else {
    let movementHandler = function(event) {
      let currentPos = Keys.getPointer() || { x: 0, y: 0};
      Keys.mousePos(
        event.movementX + currentPos.x,
        event.movementY + currentPos.y);
    };
    canvas.onclick = canvas.requestPointerLock;
    document.addEventListener('pointerlockchange', function() {
      if (document.pointerLockElement === canvas) {
        document.addEventListener('mousemove', movementHandler, false);
      } else {
        document.removeEventListener('mousemove', movementHandler);
      }
    });
  }
  canvas.addEventListener('touchstart', function(event) { Keys.touchPos(event.targetTouches[0].clientX, event.targetTouches[0].clientY); }, false);
  canvas.addEventListener('touchmove', function(event) { Keys.touchPos(event.targetTouches[0].clientX, event.targetTouches[0].clientY); }, false);
  canvas.addEventListener('touchend', function(event) {
    Keys.unTouch();
  }, false);
  canvas.addEventListener('touchcancel', function(event) {
    Keys.unTouch();
  }, false);

  log('Event handlers added', 1);

  var app = new appC(gl, Keys);
  requestAnimationFrame(app.loop);
}

