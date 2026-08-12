function detectMediaType(url) {
  const imageExtensions = 
    ['jpg', 'jpeg', 'png', 'gif', 'webp', 'bmp', "tiff", "avif"];
  const videoExtensions = 
    ['mp4', 'webm', 'ogg', 'mov', 'flv', "avi"];
  
  const extension = 
    url.split('.').pop().trim().toLowerCase();

  if (imageExtensions.includes(extension)) return 'image';
  if (videoExtensions.includes(extension)) return 'video';
  return 'other';
}

function renderAttachment(parent, data) {
	const template = document.getElementById("attachment-template");
	const li = template.content
		.cloneNode(true)
		.querySelector("li");
	const type = detectMediaType(data["name"]);
	const url = `/static/${data["name"]}`;
	switch (type) {
		case "image":
			const img = document.createElement("img");
			img.src = url;
			li.replaceChildren(img);
			break;
		case "video":
			const video = document.createElement("video");
			video.src = url;
			video.controls = true;
			li.replaceChildren(video);
			break;
		case "other":
			li.querySelector("a").href = url;
	}
	parent.appendChild(li);
}

function renderAttachments(element) {
	const attachments = JSON.parse(element.dataset.json);
	attachments.forEach((a) => {
		renderAttachment(element, a);
	});
}

document.addEventListener("DOMContentLoaded", () => {
	const elements = document.querySelectorAll(".attachments");
	elements.forEach((elem) => {
		renderAttachments(elem);
	});
});