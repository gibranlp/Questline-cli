#!/usr/bin/env python3
# ─────────────────────────────────────────────────────────────────────────────
# levelup.py — utilidad de dev para subir el personaje al máximo y desbloquear todo
# ─────────────────────────────────────────────────────────────────────────────
import os
import sys
import sqlite3
from datetime import datetime, timezone

# Busca la DB en el lugar correcto según el OS — Windows en AppData, resto en ~/.config
def get_db_path():
    if os.name == 'nt':
        appdata = os.environ.get('APPDATA')
        if appdata:
            path = os.path.join(appdata, 'questline', 'questline.db')
            if os.path.exists(path):
                return path
    
    home = os.path.expanduser('~')
    return os.path.join(home, '.config', 'questline', 'questline.db')

# The main event — conecta a la DB y lanza todos los cheats de una vez
def main():
    db_path = get_db_path()
    print(f"Target Database: {db_path}")

    if not os.path.exists(db_path):
        print(f"Error: Database file does not exist at {db_path}")
        print("Please run Questline at least once to initialize the database.")
        sys.exit(1)

    try:
        conn = sqlite3.connect(db_path)
        cursor = conn.cursor()

        print("Updating user level to 100 and experience to 500000...")
        cursor.execute("UPDATE users SET level = 100, xp = 500000;")

        print("Maxing out Zen Tree growth and health...")
        cursor.execute("UPDATE zen_tree SET stage = 5, growth = 100, health = 100;")

        print("Setting active and best streaks...")
        cursor.execute("UPDATE streaks SET current_streak = 99, best_streak = 99;")

        print("Unlocking all achievements...")
        now_str = datetime.now(timezone.utc).strftime('%Y-%m-%dT%H:%M:%SZ')
        cursor.execute("UPDATE achievements SET unlocked_at = ? WHERE unlocked_at IS NULL;", (now_str,))

        print("Completing all milestones...")
        cursor.execute("UPDATE milestones SET completed = 1;")

        # Verificar qué categorías existen antes de limpiar — para no borrar lo que no hay
        cursor.execute("SELECT DISTINCT category FROM lore_library;")
        existing_categories = cursor.fetchall()
        print(f"Existing lore categories in DB: {[r[0] for r in existing_categories]}")

        # Borrar el lore viejo e insertar las 10 historias del mundo desde cero
        print("Cleaning up old World lore and inserting the new chapters...")
        cursor.execute("DELETE FROM lore_library WHERE LOWER(category) = 'world';")

        world_chapters = [
            ("world_chapter_1", "Before the First Cursor", "Before there were projects, before there were tasks, before there were notes and chronicles, there was only the Void.\n\nThe Void was not empty.\n\nIt was filled with unfinished intentions.\n\nIdeas that would someday be started.\n\nPlans that would someday be organized.\n\nGoals that would someday be pursued.\n\nYet none of them ever moved beyond possibility.\n\nNothing was recorded.\n\nNothing was completed.\n\nNothing endured.\n\nThis forgotten era became known as The Age of Intention."),
            ("world_chapter_2", "The Age of Open Tabs", "As civilization advanced, the people attempted to impose order upon their lives.\n\nThey created notes.\n\nThey created lists.\n\nThey created plans.\n\nUnfortunately, they created them everywhere.\n\nEntire kingdoms buried themselves beneath scattered notebooks, forgotten documents, abandoned projects, and browser windows numbering in the hundreds.\n\nScholars estimate that some individuals maintained over seventy active tabs simultaneously.\n\nFew survived.\n\nHistorians still debate whether productivity truly existed during this period."),
            ("world_chapter_3", "The Rise of the Great Backlog", "Every unfinished task leaves behind a trace.\n\nA postponed promise.\n\nA ignored responsibility.\n\nA project that would be completed \"later.\"\n\nFor centuries these fragments accumulated.\n\nEventually they combined into something terrible.\n\nThe Great Backlog.\n\nNo one knows its true form.\n\nSome describe a mountain.\n\nOthers a storm.\n\nStill others claim it resembles an email inbox with twenty-seven thousand unread messages.\n\nWhatever its nature, its influence spread throughout the world.\n\nProjects stalled.\n\nDeadlines collapsed.\n\nEntire organizations vanished beneath the weight of unfinished work."),
            ("world_chapter_4", "The First Cursor", "During the darkest days of the Great Backlog, a lone traveler discovered a blinking light within the Void.\n\nIt appeared as a single cursor.\n\nPatient.\n\nSilent.\n\nWaiting.\n\nThe traveler approached.\n\nThe cursor blinked.\n\nThe traveler blinked.\n\nNeither moved for some time.\n\nEventually the traveler spoke the words:\n\n\"Hello World.\"\n\nLight spread through the darkness.\n\nStructure emerged.\n\nProjects took form.\n\nNotes gained permanence.\n\nTasks acquired purpose.\n\nThe First Cursor had awakened."),
            ("world_chapter_5", "The Founding of Questline", "In the years that followed, the surviving peoples gathered around the teachings of the First Cursor.\n\nThey learned that progress was not achieved through motivation.\n\nProgress was achieved through repetition.\n\nThrough systems.\n\nThrough consistency.\n\nThrough showing up again tomorrow.\n\nThese teachings became known as the Questline.\n\nNot because the path was easy.\n\nBut because every meaningful achievement was composed of smaller quests completed one after another."),
            ("world_chapter_6", "The Six Great Orders", "As Questline spread across the realm, different groups interpreted its teachings in different ways.\n\nSome pursued structure.\n\nOthers sought discipline.\n\nSome mastered knowledge.\n\nOthers mastered time itself.\n\nFrom these philosophies emerged the Six Great Orders.\n\nThe Arch Accountants.\n\nThe Code Warlocks.\n\nThe Mind Sages.\n\nThe Task Paladins.\n\nThe Systems Architects.\n\nThe Time Chronomancers.\n\nThough their methods differed, all sought the same goal:\n\nTo bring order to chaos and purpose to effort."),
            ("world_chapter_7", "The Era of Productivity", "For the first time in history, progress became measurable.\n\nProjects were completed.\n\nGoals were achieved.\n\nKnowledge was preserved.\n\nEntire cities prospered under the guidance of the Orders.\n\nYet success created new challenges.\n\nScope Dragons multiplied.\n\nMeeting Mimics infiltrated institutions.\n\nDeadline Wraiths appeared in increasing numbers.\n\nThe struggle against chaos had entered a new age."),
            ("world_chapter_8", "The Growth of The Evergrowth", "During this period, a mysterious sapling appeared near the center of the realm.\n\nNo one knows who planted it.\n\nNo one knows where it came from.\n\nAttempts to accelerate its growth failed.\n\nAttempts to manipulate it failed.\n\nAttempts to place it in a productivity framework generated seventeen conflicting methodologies and three conference talks.\n\nThe Tree ignored them all.\n\nIt grew only through consistent effort.\n\nA little each day.\n\nNever quickly.\n\nNever dramatically.\n\nYet never stopping."),
            ("world_chapter_9", "The Age of Chronicles", "The greatest weakness of mortals had always been memory.\n\nVictories were forgotten.\n\nProgress went unnoticed.\n\nGrowth became invisible.\n\nTo solve this, the Orders created the Chronicle.\n\nA living record of journeys, achievements, failures, discoveries, and lessons learned.\n\nThe Chronicle does not celebrate perfection.\n\nIt celebrates persistence.\n\nEvery completed task.\n\nEvery finished project.\n\nEvery return after a difficult day.\n\nAll are recorded."),
            ("world_chapter_10", "The Present Age", "The realm now stands in an era unlike any before it.\n\nThe Great Backlog remains beyond the horizon.\n\nDeadline Wraiths continue to roam.\n\nScope Dragons still tempt adventurers with promises of \"just one more feature.\"\n\nYet the people endure.\n\nEvery day new travelers begin their journey.\n\nEvery day new quests are completed.\n\nEvery day another page is added to the Chronicle.\n\nThe story of Questline remains unfinished.\n\nAs all good stories should.")
        ]

        for cid, title, content in world_chapters:
            cursor.execute(
                "INSERT OR REPLACE INTO lore_library (id, category, title, content, unlocked, unlocked_at) VALUES (?, 'World', ?, ?, 1, ?);",
                (cid, title, content, now_str)
            )

        # Lore de las clases — las 6 órdenes con su historia, rivalidades y tradiciones
        cursor.execute("DELETE FROM lore_library WHERE LOWER(category) = 'class';")

        class_stories = [
            ("class_six_orders", "The Six Great Orders", "Though all followers of Questline seek progress, few agree on how it should be achieved.\n\nOver the centuries, six distinct philosophies emerged.\n\nEach became an Order.\n\nEach mastered a different aspect of productivity.\n\nEach insists they are obviously correct.\n\nThe resulting arguments have lasted for generations.", 1),
            ("class_council_orders", "The Council of Orders", "Though the Orders often disagree, they meet each year at the Hall of Progress.\n\nRepresentatives gather to discuss threats facing the realm.\n\nThe Great Backlog.\n\nScope Dragons.\n\nDeadline Wraiths.\n\nMeeting Mimics.\n\nNotification Sprites.\n\nAnd other horrors.\n\nThe meetings usually begin with noble intentions.\n\nThey usually end with action items.\n\nThe action items are recorded.\n\nAssigned.\n\nPrioritized.\n\nScheduled.\n\nCategorized.\n\nLinked to supporting documentation.\n\nAnd occasionally completed.", 1),
            # Arch Accountant
            ("class_accountant_5", "The Order of the Ledger", "The Arch Accountants\n\n\"If it is not recorded, it did not happen.\"\n\nThe Arch Accountants were among the first followers of the Questline.\n\nWhere others sought glory, they sought balance.\n\nWhere others chased inspiration, they chased documentation.\n\nWhere others asked \"Can we afford this?\"\n\nThe Arch Accountants replied:\n\n\"We could have answered that three months ago if someone had updated the spreadsheet.\"", 1),
            ("class_accountant_15", "Business Purposes", "Their temples are vast halls of ledgers, records, receipts, reports, and financial histories stretching back centuries.\n\nEvery transaction is preserved.\n\nEvery expense is categorized.\n\nEvery discrepancy is investigated.\n\nEspecially the suspicious charge labeled:\n\n\"Business Purposes.\"\n\nNo one has ever successfully explained a Business Purposes expense to an Arch Accountant.", 1),
            ("class_accountant_20", "Traditions", "New initiates must perform the Rite of Reconciliation.\n\nA sacred ceremony in which a financial statement refuses to balance by exactly $0.03.\n\nThe ritual continues until the discrepancy is found.\n\nSome initiates emerge wiser.\n\nOthers emerge with eye twitches.", 1),
            ("class_accountant_30", "Rivalries", "Arch Accountants maintain a long-standing rivalry with Code Warlocks.\n\nThe Accountants claim developers spend money recklessly.\n\nThe Warlocks claim budgets are imaginary.\n\nBoth sides are technically correct.", 1),
            # Code Warlock
            ("class_warlock_5", "The Terminal Covenant", "The Code Warlocks\n\n\"It worked on my machine.\"\n\nNo one knows exactly how the Code Warlocks began.\n\nTheir own records are incomplete.\n\nMostly because they forgot to back them up.\n\nAccording to legend, the first Code Warlock discovered an ancient terminal hidden beneath the ruins of a forgotten data center.\n\nWithin it were the Sacred Commands.\n\nMany were dangerous.\n\nSeveral were undocumented.\n\nOne simply read:\n\nsudo trust_me\n\nHistory records that this was a mistake.", 1),
            ("class_warlock_15", "The Great Forking", "The most famous event in Warlock history was The Great Forking.\n\nA disagreement regarding indentation escalated into a civil war.\n\nEntire repositories split apart.\n\nFriendships ended.\n\nThree documentation teams disappeared.\n\nTo this day no one remembers the original argument.\n\nOnly that it was important.", 1),
            ("class_warlock_20", "Traditions", "Code Warlocks consume sacred caffeinated beverages before performing major rituals.\n\nThe stronger the coffee, the more powerful the magic.\n\nThis belief remains scientifically unchallenged.", 1),
            ("class_warlock_30", "Rivalries", "Code Warlocks and Systems Architects have argued for centuries.\n\nWarlocks believe systems should emerge naturally.\n\nArchitects believe systems should be designed beforehand.\n\nThe resulting meetings are responsible for approximately 14% of all productivity losses in recorded history.", 1),
            # Mind Sage
            ("class_sage_5", "The Silent Archive", "The Mind Sages\n\n\"That reminds me of a note I took six years ago.\"\n\nThe Mind Sages dedicate themselves to preserving knowledge.\n\nNothing is too small to record.\n\nNothing is too obscure to catalog.\n\nNothing is too ridiculous to link to three related concepts.\n\nTheir archives contain billions of interconnected ideas.\n\nMany visitors become permanently lost.\n\nFortunately, the Sages have detailed maps explaining how to escape.\n\nUnfortunately, those maps require reading seventeen prerequisite notes.", 1),
            ("class_sage_15", "The Great Linking", "A legendary Sage once connected every note in the Archive to every other note.\n\nThe resulting structure became so complex that it achieved sentience.\n\nThe Archive still occasionally recommends books no one remembers writing.", 1),
            ("class_sage_20", "Traditions", "Initiates are given a single blank page.\n\nTheir task is simple:\n\nWrite something worth remembering.\n\nMost spend years preparing.\n\nSome never begin.\n\nA few immediately write:\n\n\"Don't overthink this.\"\n\nThese individuals are usually promoted.", 1),
            ("class_sage_30", "Rivalries", "Mind Sages secretly believe everyone else's systems would improve if they simply took better notes.\n\nEveryone else secretly fears they may be right.", 1),
            # Task Paladin
            ("class_paladin_5", "The Sacred Checklist", "The Task Paladins\n\n\"Finish what you start.\"\n\nThe Task Paladins are the defenders of execution.\n\nWhile others debate.\n\nWhile others plan.\n\nWhile others research.\n\nTask Paladins complete things.\n\nThey maintain that motivation is unreliable.\n\nDiscipline is dependable.\n\nAnd checking a box feels incredible.", 1),
            ("class_paladin_15", "The Endless List", "At the center of their Order lies a stone tablet known as The Endless List.\n\nEvery unfinished task in existence is said to appear upon its surface.\n\nFortunately, the tablet is several kilometers tall.\n\nOtherwise morale would suffer considerably.", 1),
            ("class_paladin_20", "Traditions", "Young Paladins swear the Oath of Completion.\n\nThe oath is simple:\n\n\"I will stop creating new projects before finishing old ones.\"\n\nVery few survive their first year.", 1),
            ("class_paladin_30", "Rivalries", "Task Paladins view Scope Dragons as their natural enemies.\n\nUnfortunately, Scope Dragons often disguise themselves as exciting opportunities.\n\nThis has resulted in numerous tragic incidents.", 1),
            # Systems Architect
            ("class_architect_5", "The Builders of Order", "The Systems Architects\n\n\"Let's step back and look at the bigger picture.\"\n\nNo phrase has ever inspired more hope and fear simultaneously.\n\nSystems Architects see patterns where others see chaos.\n\nProcesses where others see confusion.\n\nStructure where others see piles of unrelated documents.\n\nThey possess an almost supernatural ability to create organization.\n\nMany are capable of producing folder hierarchies before understanding the project itself.", 1),
            ("class_architect_15", "The Great Refactoring", "One Architect famously reorganized an entire kingdom.\n\nRoads were rerouted.\n\nGuilds were restructured.\n\nDepartments were merged.\n\nEverything became dramatically more efficient.\n\nNo one could find anything for six months.", 1),
            ("class_architect_20", "Traditions", "Architects spend years studying the Sacred Frameworks.\n\nEvery generation eventually invents a new framework.\n\nEvery generation claims it solves all previous problems.\n\nHistory suggests otherwise.", 1),
            ("class_architect_30", "Rivalries", "Architects often clash with Task Paladins.\n\nPaladins want action.\n\nArchitects want planning.\n\nTogether they accidentally create functional organizations.", 1),
            # Time Chronomancer
            ("class_chronomancer_5", "The Keepers of Hours", "The Time Chronomancers\n\n\"That meeting could have been an email.\"\n\nThe Time Chronomancers study the most precious resource in existence:\n\nTime.\n\nUnlike gold, time cannot be earned.\n\nUnlike knowledge, time cannot be stored.\n\nUnlike tasks, time refuses to wait.\n\nChronomancers dedicate their lives to understanding where it goes.\n\nMost discoveries are deeply unsettling.", 1),
            ("class_chronomancer_15", "The Lost Afternoon", "Among their greatest mysteries is The Lost Afternoon.\n\nA temporal anomaly affecting productivity across the realm.\n\nVictims sit down for five minutes.\n\nThree hours vanish.\n\nNo explanation has ever been found.\n\nResearchers suspect social media.", 1),
            ("class_chronomancer_20", "Traditions", "Chronomancer apprentices carry hourglasses at all times.\n\nNot because they are useful.\n\nBecause it looks extremely impressive.", 1),
            ("class_chronomancer_30", "Rivalries", "Chronomancers frequently argue with Mind Sages.\n\nChronomancers believe notes should be brief.\n\nMind Sages believe brevity is reckless.\n\nThese debates often last several hours.\n\nWhich greatly annoys the Chronomancers.", 1),
        ]

        for cid, title, content, unlocked in class_stories:
            cursor.execute(
                "INSERT OR REPLACE INTO lore_library (id, category, title, content, unlocked, unlocked_at) VALUES (?, 'Class', ?, ?, ?, ?);",
                (cid, title, content, unlocked, now_str if unlocked else None)
            )

        print("Unlocking all remaining Library Lore entries...")
        cursor.execute("UPDATE lore_library SET unlocked = 1, unlocked_at = ? WHERE unlocked = 0 OR unlocked IS NULL;", (now_str,))

        conn.commit()
        conn.close()
        print("Success! Character leveled to 100, all lore entries and milestones unlocked.")
        print("Start Questline to see the changes.")
    except Exception as e:
        print(f"An error occurred while updating the database: {e}")
        sys.exit(1)

if __name__ == "__main__":
    main()
