document.addEventListener("DOMContentLoaded", () => {
	const contents = document.querySelectorAll(".content div");
	const md = new window.markdownit({
		breaks: true,
		linkify: true,
	});
	contents.forEach((elem) => {
		elem.innerHTML = md.render(elem.textContent);
	});
	hljs.highlightAll();
});