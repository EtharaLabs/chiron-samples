get_filename_component(directory "${OUTPUT}" DIRECTORY)
file(MAKE_DIRECTORY "${directory}")
file(COPY_FILE "${INPUT}" "${OUTPUT}")
file(CHMOD "${OUTPUT}" PERMISSIONS OWNER_READ OWNER_WRITE GROUP_READ WORLD_READ)
