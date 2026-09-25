# Brand images

Store brand image files in this directory. Every code-first `BrandSeed.image_name` must exactly match a file here; the seed tests enforce that relationship.

Use lowercase, filesystem-safe names such as `porsche.svg`. The database stores only the filename in `imageName`, never image bytes or an absolute machine path.

`placeholder.svg` is temporary and should be replaced by approved brand artwork before release.

