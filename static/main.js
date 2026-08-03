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

function renderAttachment(element) {
	const type = detectMediaType(element.dataset.url);
	switch (type) {
		case "image":
			const img = document.createElement("img");
			img.src = element.dataset.url;
			element.appendChild(img);
			element.open = true;
			break;
		case "video":
			const video = document.createElement("video");
			video.src = element.dataset.url;
			video.controls = true;
			element.appendChild(video);
			element.open = true;
			break;
		case "other":
			null;
	}
}

document.addEventListener("DOMContentLoaded", () => {
	const elements = document.querySelectorAll(".attachment");
	elements.forEach((elem) => {
		renderAttachment(elem);
	});
});