variable "TAG_BASE" {}
variable "RELEASE_VERSION" {}

group "default" {
	targets = ["automation"]
}

target "docker-metadata-action" {}

target "automation" {
	inherits = ["docker-metadata-action"]
	context = "./"
	dockerfile = "Dockerfile"
	tags = [for tag in target.docker-metadata-action.tags : "${TAG_BASE}:${tag}"]
	args = {
		RELEASE_VERSION="${RELEASE_VERSION}"
	}
}
