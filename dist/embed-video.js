const storeKey = "allowCookies"

function allowCookies() {
  localStorage.setItem(storeKey, true)
  renderCookieContent()
}

function renderContent(id, content) {
  let element = document.getElementById(id)
  if (element) {
    if (localStorage.getItem(storeKey)) {
      element.innerHTML = content()
    } else {
      element.innerHTML = `
        <div class="require-cookies">
          This content requires cookies
          <button onclick="allowCookies()">Allow cookies</button>
        </div>`
    }
  }
}

function video(id) {
    return `
      <iframe
        src="https://www.youtube.com/embed/${id}"
        title="YouTube video player"
        frameborder="0"
        allow="accelerometer; clipboard-write; encrypted-media; gyroscope; picture-in-picture; web-share"
        referrerpolicy="strict-origin-when-cross-origin"
        allowfullscreen
      ></iframe>`
  }

function renderCookieContent() {
  renderContent("slow-set-video", function() { return video("zD7FVVj59O8") })
}

renderCookieContent()
