"""A normal runtime package must not require image-backed movie evidence."""
import unittest
from tools.package_media import movie_evidence

class MediaEvidence(unittest.TestCase):
    def test_no_media_is_an_honest_runtime_package(self):
        self.assertEqual(movie_evidence({}), {'source':'none','verified':False})

    def test_installed_movies_do_not_require_an_image(self):
        evidence = dict(loose_movie_files_verified=True, movie_video_skip_to_ship_verified=True,
                        movie_audio_mixed_verified=True)
        self.assertEqual(movie_evidence(evidence), {'source':'installed','verified':True})

    def test_partial_or_unsourced_movie_claims_are_rejected(self):
        for evidence in [dict(movie_video_skip_to_ship_verified=True),
                         dict(cd_image_files_verified=True),dict(loose_movie_files_verified=True)]:
            with self.assertRaises(ValueError): movie_evidence(evidence)

    def test_optional_image_evidence_still_works(self):
        evidence = dict(cd_image_files_verified=True, movie_video_skip_to_ship_verified=True,
                        movie_audio_mixed_verified=True)
        self.assertEqual(movie_evidence(evidence), {'source':'image','verified':True})

    def test_installed_gog_evidence_does_not_require_a_media_argument(self):
        evidence = dict(installed_gog_movies_verified=True, movie_video_skip_to_ship_verified=True,
                        movie_audio_mixed_verified=True)
        self.assertEqual(movie_evidence(evidence), {'source':'installed-gog','verified':True})

if __name__ == '__main__': unittest.main()
