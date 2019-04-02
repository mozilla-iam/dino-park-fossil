resource "aws_s3_bucket" "cis_avatars_bucket" {
  bucket = "cis-avatars-${var.environment}"
  acl    = "private"

  tags = {
    Name        = "cis-avatars"
    Environment = "${var.environment}"
  }
}

resource "aws_iam_role" "dino_park_fossil_role" {
  name = "dino-park-fossil-role-${var.environment}-${var.region}"

  assume_role_policy = <<EOF
{
   "Version": "2012-10-17",
   "Statement": [
     {
      "Effect": "Allow",
      "Principal": {
       "Service": "ec2.amazonaws.com"
      },
      "Action": "sts:AssumeRole"
     },
     {
      "Effect": "Allow",
      "Principal": {
        "AWS": "arn:aws:iam::${data.aws_caller_identity.current.account_id}:role/kubernetes-prod-us-west-220181206181410238800000005"
       },
       "Action": "sts:AssumeRole"
      }
   ]
}
EOF
}

resource "aws_iam_role_policy" "cis_avatar_bucket_access" {
  name        = "cis-avatar-bucket-access-${var.environment}-${var.region}"
  role        = "${aws_iam_role.dino_park_fossil_role.id}"

  policy      = <<EOF
{
    "Version": "2012-10-17",
    "Statement": [
        {
            "Effect": "Allow",
            "Action": [
                "s3:PutObject",
                "s3:GetObject",
                "s3:DeleteObject"
            ],
            "Resource": "${aws_s3_bucket.cis_avatars_bucket.arn}/*"
        }
    ]
}
EOF
}
