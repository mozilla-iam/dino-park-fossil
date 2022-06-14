variable "project_name" {
  description = "DinoPark Fossil - DinoParks picture service"
  default     = "dino-park-fossil"
}

variable "github_repo" {
  default = "https://github.com/mozilla-iam/dino-park-fossil"
}

variable "buildspec_file" {
  description = "Path and name of your builspec file"
  default     = "buildspec.yml"
}

# Find all the supported images by AWS here: 
# https://docs.aws.amazon.com/codebuild/latest/userguide/build-env-ref-available.html
variable "build_image" {
  default = "aws/codebuild/standard:4.0"
}

