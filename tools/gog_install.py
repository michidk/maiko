"""Stage the owned installed GOG layout for isolated runtime checks."""
from pathlib import Path
from .toolchain import MOUNT_ROOT, is_mountable

def movie_bundle(install):
    root=Path(install).resolve()
    for directory in [root,root.parent]:
        candidates=[p for p in directory.iterdir() if p.name.lower()=='game.gog' and p.is_file()]
        if len(candidates)>1:raise ValueError('Ambiguous GOG movie data: '+str(directory))
        if candidates:
            bundle=candidates[0].resolve()
            if not is_mountable(bundle):
                raise ValueError(f'Docker movie data must resolve under {MOUNT_ROOT}')
            return bundle
    raise ValueError('No installed GOG movie data beside the selected game folder')
