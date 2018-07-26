// Check that service workers are registered
if ('serviceWorker' in navigator) {
  navigator.serviceWorker.register('sw.js');
}

function navigateWithArrows(ev) {
  var nextPage = document.querySelector("a.go-next").getAttribute("href");
  var previousPage = document.querySelector("a.go-previous").getAttribute("href");
  var toc = document.querySelector("span#reader-toc a").getAttribute("href");
  switch (ev.key) {
    case "ArrowLeft":
      window.location = previousPage;
      break;
    case "ArrowRight":
      window.location = nextPage;
      break;
    case "Escape":
      window.location = toc;
      break;
  }

  ev.preventDefault();
}

document.addEventListener('keyup', navigateWithArrows);