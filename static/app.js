// Check that service workers are registered
if ('serviceWorker' in navigator) {
  navigator.serviceWorker.register('sw.js');
}

function navigateWithArrows(ev) {
  switch (ev.key) {
    case "ArrowLeft":
      var previousPage = document.querySelector("a.go-previous").getAttribute("href");
      window.location = previousPage;
      break;
    case "ArrowRight":
      var nextPage = document.querySelector("a.go-next").getAttribute("href");
      window.location = nextPage;
      break;
    case "Escape":
      var toc = document.querySelector("span#reader-toc a").getAttribute("href");
      window.location = toc;
      break;
  }
  ev.preventDefault();
}

document.addEventListener('keyup', navigateWithArrows);