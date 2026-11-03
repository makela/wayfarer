const storeKey = "acceptCookies"

function acceptCookies() {
  localStorage.setItem(storeKey, true)
  renderCookieContent()
}

function renderVideo(element) {
  const videoId = element.dataset.videoId
  if (localStorage.getItem(storeKey)) {
    element.innerHTML = video(videoId)
  } else {
    element.innerHTML = `
      <div class="require-cookies">
        This video is loaded from YouTube, which may store data on your device
        and receives your IP address.
        <button onclick="acceptCookies()">Load the video</button>
        or watch it on
        <a href="https://www.youtube.com/watch?v=${videoId}" target="_blank">YouTube</a>.
      </div>`
  }
}

function video(id) {
    return `
      <iframe
        src="https://www.youtube-nocookie.com/embed/${id}"
        title="YouTube video player"
        frameborder="0"
        allow="accelerometer; clipboard-write; encrypted-media; gyroscope; picture-in-picture; web-share"
        referrerpolicy="strict-origin-when-cross-origin"
        allowfullscreen
      ></iframe>`
  }

function renderCookieContent() {
  document.querySelectorAll(".video[data-video-id]").forEach(renderVideo)
}

renderCookieContent()
