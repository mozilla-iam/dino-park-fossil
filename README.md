# DinoPark Fossil (drawing Dinos since 2019)
[![Build Status](https://travis-ci.org/mozilla-iam/dino-park-fossil.svg?branch=master)](https://travis-ci.org/mozilla-iam/dino-park-fossil)

DinoPark Fossil is DinoPark's picture service. It handles uploads, resizing and serving of profile pictures.

It provides the following APIs:

- `POST /avatar/send/{uuid}` to upload / change a profile picture
- `POST /avatar/send/display/{uuid}` to change a thes display level of a profile picture
- `POST /avatar/get/id/{filename}` to retrieve the picture via the _filename_ (preferred)
- `POST /avatar/get/{primaryUsername}` to retrieve the picture via the primaryUsername

For now all of these APIs are internal. We will open up the retrieving endpoints soon.