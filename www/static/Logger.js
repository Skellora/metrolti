var logOutLevel = 1;
function log(logText, level) {
  if (level === undefined) { level = 2; }
  if (level > logOutLevel) { return; }

  let logBlock = document.getElementById('log');
  if (logBlock) {
    logBlock.innerHTML = logText + '<br />' + logBlock.innerHTML;
  } else {
    alert(logText);
  }
}

