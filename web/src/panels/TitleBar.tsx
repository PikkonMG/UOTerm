import { useEffect } from 'preact/hooks';

/** The title of the page: the window title of the view, with the vitals when the Interface page asks. */
export function TitleBar({ title }: { title: string }) {
  useEffect(() => {
    document.title = title;
  }, [title]);
  return null;
}
