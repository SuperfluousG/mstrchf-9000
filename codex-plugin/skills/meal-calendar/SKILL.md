---
name: meal-calendar
description: Schedule a MSTRCHF 9000 week in the user's connected calendar, with recipe ingredients and methods. Use when they ask to book, schedule or add meals to a calendar.
---

# Meals in the calendar

Read the requested week using `get_week`. A week ID is its Monday; pick days run from 0 (Monday) to 6 (Sunday). Read each recipe through `get_recipe` with the same week ID. These tools retain an ordered week's original servings and scale method quantities as well as ingredients; do not scale the returned quantities a second time.

Confirm the target calendar, which meals and their times in one message before creating events. Suggest breakfast 08:00, lunch 12:30 and dinner 18:30 in the user's time zone, with duration equal to the recipe's minutes and at least 15 minutes. Skip dates already past. If a specific calendar, range and times are already explicitly authorised in the current request, use them without asking again.

Use an available calendar connector. If none is connected, report the missing capability and provide a reviewable schedule; do not claim events were booked. Read existing events for the week first and skip matches by meal title and date. Leave unrelated events unchanged.

Create one event per authorised meal, titled `Dinner: <recipe title>` (or Breakfast/Lunch). Include the blurb, cooking time, household size, every scaled ingredient quantity including cupboard items, and the scaled method. Finish the description with `Planned with MSTRCHF 9000.` Use sentence case headings.

Report the calendar used, events created and meals skipped. Do not send invitations or notifications to other people unless the user requested that communication.
