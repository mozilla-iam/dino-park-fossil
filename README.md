
# DinoPark Fossil (drawing Dinos since 2019)
[![Build Status](https://travis-ci.org/mozilla-iam/dino-park-fossil.svg?branch=master)](https://travis-ci.org/mozilla-iam/dino-park-fossil)
![Build Status](https://codebuild.us-west-2.amazonaws.com/badges?uuid=eyJlbmNyeXB0ZWREYXRhIjoiYm5WZU0yTkR5TEMvSTVudXNpbkhCQ21FNlR5VitCRDk3d2U2d0JwU0MwcG5zQWVxUUNOZk1yMEZ4V1M5MWliTE5VdC9RdWNsb1Q4OWIwSUljaDdraUU0PSIsIml2UGFyYW1ldGVyU3BlYyI6InpENHlxdEJPRnpSVTJTM0EiLCJtYXRlcmlhbFNldFNlcmlhbCI6MX0%3D&branch=master)

DinoPark Fossil is DinoPark's picture service. It handles uploads, resizing and serving of profile pictures.

It provides the following APIs:

- `GET /avatar/get/id/{pictureName}` to retrieve the picture
- `POST /avatar/send/intermediate` to upload a new intermediate picture (will be deleted after 24h)
- (internal) `POST /internal/send/save/{uuid}` to save an intermediate profile picture to the profile
- (internal) `POST /internal/send/display/{uuid}` to change a display level of a profile picture
