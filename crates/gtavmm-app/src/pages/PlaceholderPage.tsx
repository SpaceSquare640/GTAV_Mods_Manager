import { Icon } from "../components/IconSprite";

export function PlaceholderPage({ title }: { title: string }) {
  return (
    <section className="view" data-shown="true">
      <div className="page-head">
        <div>
          <h1 className="page-title">{title}</h1>
        </div>
      </div>
      <div className="panel">
        <div className="empty-state">
          <span className="glyph">
            <Icon name="bar-chart" />
          </span>
          <h3>Not ported yet</h3>
          <p>
            This workspace exists in the HTML design mockup but hasn't been rebuilt in the
            real React app yet — see <span className="mono">16-UI-Page-Structure.md</span> for
            the plan.
          </p>
        </div>
      </div>
    </section>
  );
}
