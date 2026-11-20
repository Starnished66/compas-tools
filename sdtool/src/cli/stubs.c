#include "library/metadata/db/metadata_db.h"
#include "library/playlists/rating_writer.h"
#include <stdbool.h>

/* The artwork coordinator asks whether music is playing so it can yield to the
 * player. A command-line tool never is. */
bool audio_is_playing(void) {
    return false;
}

/* The player's rating writer queues star changes from its UI on a Linux-only
 * worker. The tool never rates songs, so nothing is ever pending: the stored
 * rating is always current and there is nothing to submit. */
int rating_writer_effective_stars(const char *path, int stored) {
    (void) path;
    return stored;
}

int rating_writer_stars(const char *path) {
    return metadata_db_song_rating_get(path);
}

void rating_writer_submit(const char *path, int stars) {
    (void) path;
    (void) stars;
}
