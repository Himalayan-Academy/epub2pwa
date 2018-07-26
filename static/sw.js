importScripts('https://storage.googleapis.com/workbox-cdn/releases/3.4.1/workbox-sw.js');

if (workbox) {
  console.log('workbox loaded');
  // cache js
  workbox.routing.registerRoute(
    new RegExp('.*\.js'),
    workbox.strategies.networkFirst()
  );

  // cache images
  workbox.routing.registerRoute(
    /\.(?:png|gif|jpg|jpeg|svg)$/,
    workbox.strategies.cacheFirst(),
  );

  // cache HTML
  workbox.routing.registerRoute(
    /\.(?:html|css)$/,
    workbox.strategies.cacheFirst(),
  );
}

