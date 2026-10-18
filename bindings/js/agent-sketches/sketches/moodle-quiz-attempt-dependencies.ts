// Moodle Quiz attempt display and response processing, based on the Question API
// documentation and the quiz attempt controller code. The sketch follows one page
// load and submission, showing how the quiz wrapper delegates per-question work.
// Sources:
//   https://moodledev.io/docs/5.0/apis/subsystems/question
//   https://github.com/moodle/moodle/blob/main/mod/quiz/attempt.php
//   https://github.com/moodle/moodle/blob/main/mod/quiz/processattempt.php
//   https://github.com/moodle/moodle/blob/main/mod/quiz/classes/quiz_attempt.php
//   https://github.com/moodle/moodle/blob/main/mod/quiz/locallib.php
import type { Sketch } from "nicevg";
import { box } from "nicevg/style/simple";

type Node = {
  id: string;
  class: "box";
  x: number;
  y: number;
  label: string;
};

const sketch = {
  nodes: [
    { id: "student", class: "box", x: 20, y: 190, label: "Student browser\nattempt page form\nquestion response fields" },
    { id: "attempt-page", class: "box", x: 310, y: 35, label: "mod/quiz/attempt.php\nload attempt + requested page\nrender question HTML" },
    { id: "process-page", class: "box", x: 310, y: 350, label: "mod/quiz/processattempt.php\ncheck login, sesskey, owner, page\nredirect by resulting status" },
    { id: "quiz-attempt", class: "box", x: 650, y: 190, label: "mod_quiz\\quiz_attempt\nget_question_usage()\nprocess_attempt() / process_submitted_actions()" },
    { id: "quba", class: "box", x: 1010, y: 190, label: "question_usage_by_activity\nslots hold question_attempts\nshared API for quiz question work" },
    { id: "plugins", class: "box", x: 1400, y: 190, label: "qtype + qbehaviour plugins\nquestion content / response format\ninteraction, grading, feedback" },
    { id: "quiz-attempts-db", class: "box", x: 650, y: 490, label: "quiz_attempts record\nid, quiz, userid, attempt, state\nlayout, uniqueid, sumgrades" },
    { id: "question-db", class: "box", x: 1080, y: 490, label: "Question engine records\nquestion_usages\nquestion_attempts + steps" },
  ],
  edges: [
    { from: "student", to: "attempt-page", label: "GET attempt + page" },
    { from: "attempt-page", to: "quiz-attempt", label: "quiz_create_attempt_handling_errors(); load page questions" },
    { from: "quiz-attempt", to: "quiz-attempts-db", label: "attempt id; load attempt state and layout" },
    { from: "quiz-attempts-db", to: "quiz-attempt", label: "uniqueid selects question usage" },
    { from: "quiz-attempt", to: "quba", label: "question usage by activity (QUBA)" },
    { from: "quba", to: "plugins", label: "render each slot using question type and behaviour" },
    { from: "plugins", to: "quba", label: "rendered question; process response actions" },
    { from: "attempt-page", to: "student", label: "HTML form with question controls" },
    { from: "student", to: "process-page", label: "POST responses + attempt/page + finish flag" },
    { from: "process-page", to: "quiz-attempt", label: "process_attempt(timenow, finish, timeup, page)" },
    { from: "quiz-attempt", to: "quba", label: "process submitted actions; grade / update states" },
    { from: "quba", to: "question-db", label: "question_engine saves usage and attempt steps" },
    { from: "quiz-attempt", to: "quiz-attempts-db", label: "save state, timefinish, sumgrades" },
    { from: "process-page", to: "student", label: "redirect to next attempt page, summary, or review" },
  ],
  style: { box },
} satisfies Sketch<Node>;

export default sketch;
