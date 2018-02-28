function loadShader(gl, type, source) {
    var shader = gl.createShader(type);
    gl.shaderSource(shader, source);
    gl.compileShader(shader);
    if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
      log('An error occurred compiling the shaders: ' + gl.getShaderInfoLog(shader), 1);
      gl.deleteShader(shader);
      return null;
    }
	  log('Shader init success!', 1);
    return shader;
}

function Shader(gl, program) {
  const self = this;

  self.use = function() {
    gl.useProgram(program);
  }

  self.getAttribLocation = function(name) {
    return gl.getAttribLocation(program, name);
  }

  const uniform = function(name) {
    return gl.getUniformLocation(program, name);
  }

  self.setUniformInt = function(name, value) {
    gl.uniform1i(uniform(name), value);
  }

  self.setUniformFloat = function(name, value) {
    gl.uniform1f(uniform(name), value);
  }

  self.setUniformVec2 = function(name, x, y) {
    gl.uniform2f(uniform(name), x, y);
  }

  self.setUniformVec3 = function(name, x, y, z) {
    gl.uniform3f(uniform(name), x, y, z);
  }

  self.setUniformVec4 = function(name, x, y, z, w) {
    gl.uniform4f(uniform(name), x, y, z, w);
  }

  self.setUniformMat4 = function(name, mat) {
    let v = new Float32Array(mat.flatten());
    gl.uniformMatrix4fv(uniform(name), false, v);
  }
}

function makeShaderProgram(gl, vertexSource, fragmentSource) {
  const vertexShader = loadShader(gl, gl.VERTEX_SHADER, vertexSource);
  const fragmentShader = loadShader(gl, gl.FRAGMENT_SHADER, fragmentSource);

  let program = gl.createProgram();
  gl.attachShader(program, vertexShader);
  gl.attachShader(program, fragmentShader);
  gl.linkProgram(program);

  gl.deleteShader(vertexShader);
  gl.deleteShader(fragmentShader);

  if (!gl.getProgramParameter(program, gl.LINK_STATUS)) {
    log('Unable to initialize the shader program: ' + gl.getProgramInfoLog(program), 1);
    return null;
  }

  return new Shader(gl, program);
}

function SetUpAttributes(gl, program, attribs, indexSize) {
  let current = 0;
  let total = 0;
  for (let i = 0; i < attribs.length; i++) {
    total += attribs[i][1];
  }
  for (let i = 0; i < attribs.length; i++) {
    let name = attribs[i][0];
    let size = attribs[i][1];
    if (name) {
      let loc = program.getAttribLocation(name);
      
      gl.vertexAttribPointer(loc, size, gl.FLOAT, false, total * indexSize, current * indexSize);
      gl.enableVertexAttribArray(loc);
    }
    current = current + size;
  }
}

