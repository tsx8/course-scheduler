import json
import uuid
from ortools.sat.python import cp_model

class DataManager:
    """A helper class to load, process, and manage the data."""
    def __init__(self, data):
        self.data = data
        self._create_mappings()
        self._preprocess_data()

    def _create_mappings(self):
        """Create ID-to-index and index-to-ID mappings for easier lookup."""
        self.time = {item['id']: item for item in self.data['time']}
        self.day = {item['id']: item for item in self.data['day']}
        self.courses = {item['id']: item for item in self.data['courses']}
        self.teachers = {item['id']: item for item in self.data['teachers']}
        
        self.time_list = self.data['time']
        self.day_list = self.data['day']
        self.course_list = self.data['courses']
        self.teacher_list = self.data['teachers']
        self.campus_list = self.data['campuses']
        
        self.venue_list = []
        for campus in self.campus_list:
            for venue in campus['venues']:
                self.venue_list.append({**venue, 'campus_id': campus['id']})

        self.time_id_to_idx = {t['id']: i for i, t in enumerate(self.time_list)}
        self.day_id_to_idx = {d['id']: i for i, d in enumerate(self.day_list)}
        self.course_id_to_idx = {c['id']: i for i, c in enumerate(self.course_list)}
        self.teacher_id_to_idx = {t['id']: i for i, t in enumerate(self.teacher_list)}
        self.venue_id_to_idx = {v['id']: i for i, v in enumerate(self.venue_list)}
        self.campus_id_to_idx = {c['id']: i for i, c in enumerate(self.campus_list)}
        
        self.venue_idx_to_campus_idx = {
            self.venue_id_to_idx[v['id']]: self.campus_id_to_idx[c['id']]
            for c in self.campus_list for v in c['venues']
        }
        
        self.density_map = {}
        for c_idx, campus in enumerate(self.campus_list):
            for density in campus.get('schedule_density', []):
                d_idx = self.day_id_to_idx[density['day_id']]
                t_idx = self.time_id_to_idx[density['time_id']]
                self.density_map[(c_idx, d_idx, t_idx)] = density['count']


    def _preprocess_data(self):
        """Pre-calculate values from existing schedules."""
        self.existing_teacher_hours = {t_idx: 0 for t_idx in range(len(self.teacher_list))}
        self.existing_venue_usage = {}
        self.existing_teacher_day_campus = {}
        self.teacher_has_existing_campus = {
            t_idx: {c_idx: False for c_idx in range(len(self.campus_list))}
            for t_idx in range(len(self.teacher_list))
        }
        self.existing_teacher_day_time_busy = {}
        
        self.existing_work_days = {t_idx: set() for t_idx in range(len(self.teacher_list))}
        
        self.existing_teacher_day_class_count = {}

        for t_idx, teacher in enumerate(self.teacher_list):
            for scheduled in teacher.get('scheduled', []):
                time_id = scheduled['time_id']
                day_id = scheduled['day_id']
                venue_id = scheduled['venue_id']
                campus_id = scheduled['campus_id']
                
                hours = self.time[time_id]['corresponding_hours']
                self.existing_teacher_hours[t_idx] += hours
                
                d_idx = self.day_id_to_idx[day_id]
                i_idx = self.time_id_to_idx[time_id]
                v_idx = self.venue_id_to_idx[venue_id]
                c_idx = self.campus_id_to_idx[campus_id]
                
                count_key = (t_idx, d_idx)
                self.existing_teacher_day_class_count[count_key] = self.existing_teacher_day_class_count.get(count_key, 0) + 1
                
                self.existing_work_days[t_idx].add(d_idx)

                key = (v_idx, d_idx, i_idx)
                self.existing_venue_usage[key] = self.existing_venue_usage.get(key, 0) + 1
                
                if (t_idx, d_idx) in self.existing_teacher_day_campus and self.existing_teacher_day_campus[(t_idx, d_idx)] != c_idx:
                    print(f"ERROR: Teacher {teacher['name']} has conflicting existing schedules on the same day across different campuses.")
                self.existing_teacher_day_campus[(t_idx, d_idx)] = c_idx
                self.teacher_has_existing_campus[t_idx][c_idx] = True
                
                self.existing_teacher_day_time_busy[(t_idx, d_idx, i_idx)] = True

def solve_scheduling(data_manager):
    """Builds and solves the CP-SAT model."""
    dm = data_manager
    model = cp_model.CpModel()

    # --- Decision Variables ---
    x = {}
    
    for t_idx, teacher in enumerate(dm.teacher_list):
        for course_id in teacher['teaches']:
            c_idx = dm.course_id_to_idx[course_id]
            course = dm.course_list[c_idx]
            for place in course['place']:
                v_idx = dm.venue_id_to_idx[place['venue_id']]
                for d_idx in range(len(dm.day_list)):
                    for i_idx in range(len(dm.time_list)):
                        key = (t_idx, c_idx, d_idx, i_idx, v_idx)
                        x[key] = model.NewBoolVar(f'x_{t_idx}_{c_idx}_{d_idx}_{i_idx}_{v_idx}')

    print(f"Created {len(x)} potential assignment variables after pruning.")

    # --- Hard Constraints ---

    # A teacher can teach at most one class at any given time.
    for t_idx, teacher in enumerate(dm.teacher_list):
        teacher_id = teacher['id']
        unavailable_slots = {(dm.day_id_to_idx[u['day_id']], dm.time_id_to_idx[u['time_id']]) for u in teacher.get('unavailable', [])}
        for d_idx in range(len(dm.day_list)):
            for i_idx in range(len(dm.time_list)):
                
                possible_classes_in_slot = [
                    var for key, var in x.items()
                    if key[0] == t_idx and key[2] == d_idx and key[3] == i_idx
                ]
                
                if dm.existing_teacher_day_time_busy.get((t_idx, d_idx, i_idx), False):
                    model.Add(sum(possible_classes_in_slot) == 0)
                else:
                    model.Add(sum(possible_classes_in_slot) <= 1)

                if (d_idx, i_idx) in unavailable_slots:
                    model.Add(sum(possible_classes_in_slot) == 0)
    
    # A venue cannot be overbooked.
    for v_idx in range(len(dm.venue_list)):
        venue_capacity = dm.venue_list[v_idx]['capacity']
        for d_idx in range(len(dm.day_list)):
            for i_idx in range(len(dm.time_list)):
                existing_usage = dm.existing_venue_usage.get((v_idx, d_idx, i_idx), 0)
                new_usage = sum(
                    var for key, var in x.items()
                    if key[4] == v_idx and key[2] == d_idx and key[3] == i_idx
                )
                model.Add(new_usage + existing_usage <= venue_capacity)

    # A teacher cannot exceed their maximum teaching hours.
    for t_idx, teacher in enumerate(dm.teacher_list):
        existing_hours = dm.existing_teacher_hours[t_idx]
        new_hours = sum(
            var * dm.time_list[key[3]]['corresponding_hours'] 
            for key, var in x.items() if key[0] == t_idx
        )
        model.Add(new_hours + existing_hours <= teacher['max_teaching_hours'])

    # "is_only_shahe" teachers can only teach at the Shahe campus.
    for t_idx, teacher in enumerate(dm.teacher_list):
        if teacher['is_only_shahe']:
            shahe_campus_idx = dm.campus_id_to_idx['138697dc-1591-4c16-b60e-d0057964be56']
            for key, var in x.items():
                if key[0] == t_idx:
                    venue_campus_idx = dm.venue_idx_to_campus_idx[key[4]]
                    model.AddImplication(var, model.NewConstant(venue_campus_idx == shahe_campus_idx))

    # A teacher can only be at one campus on any given day.
    for t_idx in range(len(dm.teacher_list)):
        for d_idx in range(len(dm.day_list)):
            teacher_day_campus_vars = [model.NewBoolVar(f'teacher_{t_idx}_day_{d_idx}_campus_{c_idx}') for c_idx in range(len(dm.campus_list))]
            model.Add(sum(teacher_day_campus_vars) <= 1)
            
            if (t_idx, d_idx) in dm.existing_teacher_day_campus:
                fixed_campus_idx = dm.existing_teacher_day_campus[(t_idx, d_idx)]
                for c_idx, campus_var in enumerate(teacher_day_campus_vars):
                    model.Add(campus_var == (c_idx == fixed_campus_idx))

            for key, var in x.items():
                if key[0] == t_idx and key[2] == d_idx:
                    venue_campus_idx = dm.venue_idx_to_campus_idx[key[4]]
                    model.AddImplication(var, teacher_day_campus_vars[venue_campus_idx])

    # Teachers not restricted to Shahe must teach at both campuses.
    for t_idx, teacher in enumerate(dm.teacher_list):
        if not teacher['is_only_shahe']:
            teacher_teaches_at_campus = [model.NewBoolVar(f'teacher_{t_idx}_teaches_at_campus_{c_idx}') for c_idx in range(len(dm.campus_list))]
            
            for c_idx in range(len(dm.campus_list)):
                has_existing = dm.teacher_has_existing_campus[t_idx][c_idx]
                new_classes_at_campus = [
                    var for key, var in x.items()
                    if key[0] == t_idx and dm.venue_idx_to_campus_idx[key[4]] == c_idx
                ]
                model.AddBoolOr([model.NewConstant(has_existing)] + new_classes_at_campus).OnlyEnforceIf(teacher_teaches_at_campus[c_idx])
                
            model.Add(sum(teacher_teaches_at_campus) == len(dm.campus_list))

    # This dictionary will store the working day variables for each teacher,
    # to be reused by the soft constraint for workday concentration.
    all_teacher_work_day_vars = {}
    
    # Teachers should not work more than 3 days a week, unless already scheduled for more.
    for t_idx, teacher in enumerate(dm.teacher_list):
        num_existing_days = len(dm.existing_work_days[t_idx])

        if num_existing_days >= 4:
            for d_idx in range(len(dm.day_list)):
                if d_idx not in dm.existing_work_days[t_idx]:
                    new_classes_on_this_day = [
                        var for key, var in x.items()
                        if key[0] == t_idx and key[2] == d_idx
                    ]
                    if new_classes_on_this_day:
                        model.Add(sum(new_classes_on_this_day) == 0)
            continue 

        is_working_day_vars = []
        for d_idx in range(len(dm.day_list)):
            works_on_day = model.NewBoolVar(f'teacher_{t_idx}_works_on_day_{d_idx}')
            is_working_day_vars.append(works_on_day)

            new_classes_on_day = [
                var for key, var in x.items()
                if key[0] == t_idx and key[2] == d_idx
            ]
            has_existing_on_day = 1 if d_idx in dm.existing_work_days[t_idx] else 0

            total_activity_on_day = sum(new_classes_on_day) + has_existing_on_day

            model.Add(total_activity_on_day > 0).OnlyEnforceIf(works_on_day)
            
            model.Add(total_activity_on_day == 0).OnlyEnforceIf(works_on_day.Not())

        all_teacher_work_day_vars[t_idx] = is_working_day_vars
        model.Add(sum(is_working_day_vars) <= 3)

    # --- Soft Constraints & Objective Function ---

    # Soft Constraint 1: Penalize having only one class on a given day.
    single_class_day_penalties = []
    for t_idx in range(len(dm.teacher_list)):
        for d_idx in range(len(dm.day_list)):
            existing_classes_count = dm.existing_teacher_day_class_count.get((t_idx, d_idx), 0)
            new_classes_on_day = sum(
                var for key, var in x.items()
                if key[0] == t_idx and key[2] == d_idx
            )
            
            total_classes_on_day = model.NewIntVar(0, 10, f'total_classes_t{t_idx}_d{d_idx}')
            model.Add(total_classes_on_day == existing_classes_count + new_classes_on_day)
            
            is_single_class_day = model.NewBoolVar(f'is_single_class_day_t{t_idx}_d{d_idx}')
            
            model.Add(total_classes_on_day == 1).OnlyEnforceIf(is_single_class_day)
            model.Add(total_classes_on_day != 1).OnlyEnforceIf(is_single_class_day.Not())
            
            single_class_day_penalties.append(is_single_class_day)
    total_single_class_penalty = sum(single_class_day_penalties)

    # Soft Constraint 2: Penalize gaps in a teacher's daily schedule for continuity.
    schedule_gap_penalties = []
    for t_idx in range(len(dm.teacher_list)):
        for d_idx in range(len(dm.day_list)):
            # 1. Create a variable for each time slot indicating if a class is scheduled.
            has_class_on_day = []
            for i_idx in range(len(dm.time_list)):
                has_class_var = model.NewBoolVar(f'has_class_t{t_idx}_d{d_idx}_i{i_idx}')
                has_class_on_day.append(has_class_var)

                new_classes_in_slot = [
                    var for key, var in x.items()
                    if key[0] == t_idx and key[2] == d_idx and key[3] == i_idx
                ]
                has_existing = dm.existing_teacher_day_time_busy.get((t_idx, d_idx, i_idx), False)

                if has_existing:
                    model.Add(has_class_var == 1)
                else:
                    if new_classes_in_slot:
                        model.Add(sum(new_classes_in_slot) > 0).OnlyEnforceIf(has_class_var)
                        model.Add(sum(new_classes_in_slot) == 0).OnlyEnforceIf(has_class_var.Not())
                    else:
                        model.Add(has_class_var == 0)

            # 2. Identify and penalize gaps. A gap is an idle slot between two busy slots.
            for i_idx in range(len(dm.time_list)):
                # Does a class exist *before* this time slot on this day?
                has_class_before = model.NewBoolVar(f'class_before_t{t_idx}_d{d_idx}_i{i_idx}')
                if i_idx > 0:
                    model.AddBoolOr(has_class_on_day[0:i_idx]).OnlyEnforceIf(has_class_before)
                    model.Add(sum(has_class_on_day[0:i_idx]) == 0).OnlyEnforceIf(has_class_before.Not())
                else:
                    model.Add(has_class_before == 0)

                # Does a class exist *after* this time slot on this day?
                has_class_after = model.NewBoolVar(f'class_after_t{t_idx}_d{d_idx}_i{i_idx}')
                if i_idx < len(dm.time_list) - 1:
                    model.AddBoolOr(has_class_on_day[i_idx + 1:]).OnlyEnforceIf(has_class_after)
                    model.Add(sum(has_class_on_day[i_idx + 1:]) == 0).OnlyEnforceIf(has_class_after.Not())
                else:
                    model.Add(has_class_after == 0)
                
                # A gap exists if there's a class before, a class after, and no class at this current slot.
                is_a_gap = model.NewBoolVar(f'is_gap_t{t_idx}_d{d_idx}_i{i_idx}')
                
                # Model is_a_gap <=> AND(has_class_before, has_class_after, NOT has_class_on_day[i_idx])
                model.Add(is_a_gap <= has_class_before)
                model.Add(is_a_gap <= has_class_after)
                model.Add(is_a_gap <= has_class_on_day[i_idx].Not())
                model.Add(is_a_gap >= has_class_before + has_class_after + has_class_on_day[i_idx].Not() - 2)

                schedule_gap_penalties.append(is_a_gap)
    total_schedule_gap_penalty = sum(schedule_gap_penalties)

    # NEW: Soft Constraint 3: Penalize scattered work days for a teacher to encourage concentration.
    workday_concentration_penalties = []
    for t_idx in range(len(dm.teacher_list)):
        # Retrieve the work-day variables defined in the hard constraint section.
        works_on_day_vars = all_teacher_work_day_vars.get(t_idx)
        if not works_on_day_vars:
            # This teacher's work days are likely fixed (e.g., already working >= 4 days), so we skip them.
            continue
        
        # Identify and penalize idle days that fall between working days.
        for d_idx in range(len(dm.day_list)):
            # Does a work day exist *before* this day in the week?
            has_work_before = model.NewBoolVar(f'work_before_t{t_idx}_d{d_idx}')
            if d_idx > 0:
                model.AddBoolOr(works_on_day_vars[0:d_idx]).OnlyEnforceIf(has_work_before)
                model.Add(sum(works_on_day_vars[0:d_idx]) == 0).OnlyEnforceIf(has_work_before.Not())
            else:
                model.Add(has_work_before == 0)

            # Does a work day exist *after* this day in the week?
            has_work_after = model.NewBoolVar(f'work_after_t{t_idx}_d{d_idx}')
            if d_idx < len(dm.day_list) - 1:
                model.AddBoolOr(works_on_day_vars[d_idx + 1:]).OnlyEnforceIf(has_work_after)
                model.Add(sum(works_on_day_vars[d_idx + 1:]) == 0).OnlyEnforceIf(has_work_after.Not())
            else:
                model.Add(has_work_after == 0)
            
            # A "gap day" is an idle day (works_on_day_vars[d_idx] is false)
            # between two work days (has_work_before and has_work_after are true).
            is_a_gap_day = model.NewBoolVar(f'is_gap_day_t{t_idx}_d{d_idx}')
            
            # Model is_a_gap_day <=> AND(has_work_before, has_work_after, NOT works_on_day_vars[d_idx])
            model.Add(is_a_gap_day <= has_work_before)
            model.Add(is_a_gap_day <= has_work_after)
            model.Add(is_a_gap_day <= works_on_day_vars[d_idx].Not())
            model.Add(is_a_gap_day >= has_work_before + has_work_after + works_on_day_vars[d_idx].Not() - 2)

            workday_concentration_penalties.append(is_a_gap_day)
            
    total_workday_concentration_penalty = sum(workday_concentration_penalties)


    # Objective 1: Minimize deviation from schedule density targets.
    diff_vars = []
    for c_idx in range(len(dm.campus_list)):
        for d_idx in range(len(dm.day_list)):
            for i_idx in range(len(dm.time_list)):
                target_count = dm.density_map.get((c_idx, d_idx, i_idx), 0)
                
                existing_count = sum(
                    count for (v, d, i), count in dm.existing_venue_usage.items()
                    if d == d_idx and i == i_idx and dm.venue_idx_to_campus_idx[v] == c_idx
                )
                
                new_count = sum(
                    var for key, var in x.items()
                    if dm.venue_idx_to_campus_idx[key[4]] == c_idx and key[2] == d_idx and key[3] == i_idx
                )
                
                actual_count = new_count + existing_count
                
                diff_var = model.NewIntVar(0, 100, f'diff_c{c_idx}_d{d_idx}_i{i_idx}')
                model.Add(target_count - actual_count <= diff_var)
                model.Add(actual_count - target_count <= diff_var)
                diff_vars.append(diff_var)

    max_deviation = model.NewIntVar(0, 100, 'max_deviation')
    model.AddMaxEquality(max_deviation, diff_vars)
    total_deviation = sum(diff_vars)

    # Objective 2: Maximize the number of "teacher-course-campus" combinations offered.
    opened_course_options = []
    for t_idx, teacher in enumerate(dm.teacher_list):
        campus_indices_to_check = range(len(dm.campus_list))
        if teacher['is_only_shahe']:
            shahe_campus_idx = dm.campus_id_to_idx['138697dc-1591-4c16-b60e-d0057964be56']
            campus_indices_to_check = [shahe_campus_idx]

        for course_id in teacher['teaches']:
            c_idx = dm.course_id_to_idx[course_id]
            for campus_idx in campus_indices_to_check:
                opens_course_at_campus = model.NewBoolVar(f'opens_t{t_idx}_c{c_idx}_k{campus_idx}')
                
                assignments_for_option = [
                    var for key, var in x.items()
                    if key[0] == t_idx and key[1] == c_idx and dm.venue_idx_to_campus_idx[key[4]] == campus_idx
                ]
                
                if not assignments_for_option:
                    model.Add(opens_course_at_campus == 0)
                    continue

                model.Add(sum(assignments_for_option) > 0).OnlyEnforceIf(opens_course_at_campus)
                model.Add(sum(assignments_for_option) == 0).OnlyEnforceIf(opens_course_at_campus.Not())
                
                opened_course_options.append(opens_course_at_campus)
    total_opened_courses = sum(opened_course_options)

    # Define weights for different parts of the objective function.
    WEIGHT_FOR_TOTAL_DEVIATION = 1
    WEIGHT_FOR_MAX_DEVIATION = 1
    WEIGHT_FOR_OPENED_COURSES = 1 
    WEIGHT_FOR_SINGLE_CLASS_PENALTY = 1
    WEIGHT_FOR_SCHEDULE_GAPS = 1
    WEIGHT_FOR_WORKDAY_CONCENTRATION = 1 # Higher weight to strongly encourage concentrated schedules

    # Combine objectives: Minimize deviation, single-class days and schedule gaps, while maximizing opened courses.
    model.Minimize(
          (WEIGHT_FOR_TOTAL_DEVIATION * total_deviation)
        + (WEIGHT_FOR_MAX_DEVIATION * max_deviation) 
        - (WEIGHT_FOR_OPENED_COURSES * total_opened_courses)
        + (WEIGHT_FOR_SINGLE_CLASS_PENALTY * total_single_class_penalty)
        + (WEIGHT_FOR_SCHEDULE_GAPS * total_schedule_gap_penalty)
        + (WEIGHT_FOR_WORKDAY_CONCENTRATION * total_workday_concentration_penalty)
    )
    
    # --- Solve and Output ---
    solver = cp_model.CpSolver()
    solver.parameters.max_time_in_seconds = 60.0
    status = solver.Solve(model)

    if status == cp_model.OPTIMAL or status == cp_model.FEASIBLE:
        print(f'Solution found with status: {"OPTIMAL" if status == cp_model.OPTIMAL else "FEASIBLE"}')
        print(f'Objective value (lower is better): {solver.ObjectiveValue()}')
        
        new_schedules = {}
        for key, var in x.items():
            if solver.Value(var) == 1:
                t_idx, c_idx, d_idx, i_idx, v_idx = key
                
                teacher_id = dm.teacher_list[t_idx]['id']
                venue = dm.venue_list[v_idx]

                new_schedule_item = {
                    "id": str(uuid.uuid4()),
                    "day_id": dm.day_list[d_idx]['id'],
                    "time_id": dm.time_list[i_idx]['id'],
                    "course_id": dm.course_list[c_idx]['id'],
                    "campus_id": venue['campus_id'],
                    "venue_id": venue['id']
                }
                
                if teacher_id not in new_schedules:
                    new_schedules[teacher_id] = []
                new_schedules[teacher_id].append(new_schedule_item)
        
        for teacher in dm.data['teachers']:
            if teacher['id'] in new_schedules:
                # Ensure 'scheduled' list exists before extending
                if 'scheduled' not in teacher:
                    teacher['scheduled'] = []
                teacher['scheduled'].extend(new_schedules[teacher['id']])
                print(f"Scheduled {len(new_schedules[teacher['id']])} new class(es) for {teacher['name']}.")

        return dm.data

    else:
        print('No solution found.')
        return None


if __name__ == '__main__':
    import argparse

    parser = argparse.ArgumentParser(description='Solve a course scheduling problem.')
    parser.add_argument('input_file', type=str, help='Path to the input data JSON file.')
    parser.add_argument('output_file', type=str, help='Path to write the output solution JSON file.')
    args = parser.parse_args()

    try:
        with open(args.input_file, 'r', encoding='utf-8') as f:
            data = json.load(f)
    except FileNotFoundError:
        print(f"Error: Input file not found at {args.input_file}")
        exit(1)

    data_manager = DataManager(data)
    
    solution_data = solve_scheduling(data_manager)

    if solution_data:
        with open(args.output_file, 'w', encoding='utf-8') as f:
            json.dump(solution_data, f, ensure_ascii=False, indent=2)
        print(f"\nSuccessfully generated schedule. See {args.output_file} for details.")
    else:
        print("\nSolver did not find a solution.")