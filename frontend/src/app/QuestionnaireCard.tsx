import { Link } from 'react-router'

import { dismissQuestionnaireCard } from './questionnaire'

/** Offers the questionnaire on the home screen until the client starts it. */
export function QuestionnaireCard({ onDismiss }: { onDismiss: () => void }) {
  return (
    <div className="prompt-card">
      <strong>Анкета</strong>
      <p className="muted small">Вік, зріст і фото твого залу — щоб програма була саме під тебе.</p>
      <div className="prompt-actions">
        <Link className="button small" to="/app/questionnaire">
          Заповнити
        </Link>
        <button
          type="button"
          className="link-button"
          onClick={() => {
            dismissQuestionnaireCard()
            onDismiss()
          }}
        >
          Не зараз
        </button>
      </div>
    </div>
  )
}
