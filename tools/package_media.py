"""Keep optional movie acceptance separate from normal runtime packaging."""

def movie_evidence(report):
    source = ('installed-gog' if report.get('installed_gog_movies_verified') else
              'installed' if report.get('loose_movie_files_verified') else
              'image' if report.get('cd_image_files_verified') else 'none')
    video = bool(report.get('movie_video_skip_to_ship_verified'))
    audio = bool(report.get('movie_audio_mixed_verified'))
    if source == 'none' and not video and not audio:
        return dict(source='none', verified=False)
    if source == 'none' or not video or not audio:
        raise ValueError('Movie claims require a tested installed-file or optional-image source, video/skip/ship and mixed PCM evidence')
    return dict(source=source, verified=True)
