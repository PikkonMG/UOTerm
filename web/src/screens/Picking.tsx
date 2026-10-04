interface PickingProps {
  title: string;
  names: string[];
  onPick(index: number): void;
}

/** A list of names to pick one from: the shards of the login. */
export function Picking({ title, names, onPick }: PickingProps) {
  return (
    <section class="panel screen">
      <h1 class="title">{title}</h1>
      {names.map((name, index) => (
        <button type="button" class="button row" key={`${index}-${name}`} onClick={() => onPick(index)}>
          {name}
        </button>
      ))}
    </section>
  );
}
