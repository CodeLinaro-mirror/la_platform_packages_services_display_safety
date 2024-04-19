#!/system/bin/sh

# TODO: Load config from RO folder, documents from another folder.
# This needs a Harry-prebuild update.

SOURCE_DIR="/vendor/etc/harry"
DATA_DIR="/data/vendor/com.google.display_safety.har"

if [ ! -d "$DATA_DIR" ]; then
  mkdir -p ${DATA_DIR}
  cp -r $SOURCE_DIR/* $DATA_DIR
else
  echo "${DATA_DIR} already exists"
fi
