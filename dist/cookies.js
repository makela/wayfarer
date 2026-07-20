const storeKey = "allowCookies"

async function allowCookies() {
  await cookieStore.set(storeKey, true)
  renderCookies()
}

async function renderCookie(id, content) {
  let element = document.getElementById(id)
  if (element) {
    let isCookiesAllowed = await cookieStore.get(storeKey)
    if (isCookiesAllowed) {
      element.innerHTML = content()
    } else {
      element.innerHTML = `
        <div class="use-cookies">
          This content requires cookies
          <button onclick="allowCookies()">Allow cookies</button>
        </div>`
    }
  }
}

async function renderCookies() {
  await renderCookie("slow-set-video", function() {
    return `
      <div class="video">
      <div class="embed-youtube">
        <iframe
          src="https://www.youtube.com/embed/zD7FVVj59O8"
          title="YouTube video player"
          frameborder="0"
          allow="accelerometer; clipboard-write; encrypted-media; gyroscope; picture-in-picture; web-share"
          referrerpolicy="strict-origin-when-cross-origin"
          allowfullscreen
        ></iframe>
      </div>
      </div>`
  })
}

renderCookies()
